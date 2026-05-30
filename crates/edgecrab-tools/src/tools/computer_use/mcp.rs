//! Dedicated stdio MCP client for `cua-driver mcp`.

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

/// Per-request deadline for a cua-driver RPC. Without this, a wedged daemon
/// makes `read_line` block indefinitely (observed as multi-minute stalls before
/// cua-driver's own internal timeout fires). Override via
/// `EDGECRAB_CUA_DRIVER_TIMEOUT_SECS`.
fn rpc_timeout() -> std::time::Duration {
    let secs = std::env::var("EDGECRAB_CUA_DRIVER_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&s| s > 0)
        .unwrap_or(60);
    std::time::Duration::from_secs(secs)
}

#[derive(Debug)]
pub struct McpToolResult {
    pub data: Value,
    pub images: Vec<String>,
    pub structured: Option<Value>,
    pub is_error: bool,
}

pub struct CuaMcpSession {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl CuaMcpSession {
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self, String> {
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn {command}: {e}"))?;

        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;

        let mut session = Self {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
        };

        session
            .rpc(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "edgecrab", "version": env!("CARGO_PKG_VERSION") }
                }),
            )
            .await?;

        session
            .send_raw(json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }))
            .await?;

        Ok(session)
    }

    async fn send_raw(&mut self, request: Value) -> Result<(), String> {
        let msg = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        self.stdin
            .write_all(msg.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| e.to_string())?;
        self.stdin.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn read_line(&mut self) -> Result<Value, String> {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if line.is_empty() {
            return Err("cua-driver closed connection".into());
        }
        serde_json::from_str(&line).map_err(|e| format!("invalid JSON from cua-driver: {e}"))
    }

    async fn rpc(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = next_id();
        self.send_raw(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))
        .await?;

        // Read until we see the response whose `id` matches this request, with an
        // overall deadline. Lines without a matching id (notifications, or stale
        // responses from an earlier timed-out request) are skipped so the stream
        // never desynchronizes.
        let timeout = rpc_timeout();
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "cua-driver request '{method}' timed out after {}s (daemon unresponsive)",
                    timeout.as_secs()
                ));
            }
            let response = match tokio::time::timeout(remaining, self.read_line()).await {
                Ok(line) => line?,
                Err(_) => {
                    return Err(format!(
                        "cua-driver request '{method}' timed out after {}s (daemon unresponsive)",
                        timeout.as_secs()
                    ));
                }
            };

            // Skip anything that isn't the response to *this* request id.
            match response.get("id").and_then(Value::as_u64) {
                Some(rid) if rid == id => {
                    if let Some(err) = response.get("error") {
                        return Err(err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("MCP error")
                            .to_string());
                    }
                    return Ok(response.get("result").cloned().unwrap_or(Value::Null));
                }
                _ => continue,
            }
        }
    }

    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<McpToolResult, String> {
        let result = self
            .rpc(
                "tools/call",
                json!({ "name": name, "arguments": args }),
            )
            .await?;
        Ok(extract_tool_result(&result))
    }
}

fn extract_tool_result(result: &Value) -> McpToolResult {
    let is_error = result.get("isError").and_then(|v| v.as_bool()).unwrap_or(false);
    let structured = result.get("structuredContent").cloned();
    let mut images = Vec::new();
    let mut text_chunks = Vec::new();
    if let Some(parts) = result.get("content").and_then(|c| c.as_array()) {
        for part in parts {
            match part.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                        text_chunks.push(t.to_string());
                    }
                }
                Some("image") => {
                    if let Some(b64) = part.get("data").and_then(|v| v.as_str()) {
                        images.push(b64.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    let joined = text_chunks.join("\n");
    let data = if joined.trim_start().starts_with('{') || joined.trim_start().starts_with('[') {
        serde_json::from_str(&joined).unwrap_or(Value::String(joined))
    } else {
        Value::String(joined)
    };
    McpToolResult {
        data,
        images,
        structured,
        is_error,
    }
}
