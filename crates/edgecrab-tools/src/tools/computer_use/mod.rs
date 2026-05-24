//! Provider-agnostic macOS desktop control via cua-driver MCP.
//!
//! Mirrors Hermes `tools/computer_use/`. macOS + cua-driver required for live use;
//! `EDGECRAB_COMPUTER_USE_BACKEND=noop` forces the test stub.

mod aux_vision;
mod backend;
mod cua_backend;
mod dispatch;
mod mcp;
mod noop;
mod permissions;
mod response;
mod safety;
mod schema;
mod status;
mod vision_routing;

#[cfg(test)]
mod tests;

use std::sync::OnceLock;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde_json::json;

use edgecrab_types::{ToolError, ToolSchema};

use crate::registry::{ToolContext, ToolHandler};

pub use permissions::{check_requirements, permissions_status};
pub use response::parse_multimodal_tool_output;
pub use status::{ComputerUseStatusConfig, format_computer_command};

static BACKEND: OnceLock<Mutex<Box<dyn backend::ComputerUseBackend>>> = OnceLock::new();

fn backend_name() -> String {
    std::env::var("EDGECRAB_COMPUTER_USE_BACKEND")
        .unwrap_or_else(|_| "cua".to_string())
        .to_ascii_lowercase()
}

#[cfg(test)]
pub fn reset_backend_for_tests() {
    // Tests set EDGECRAB_COMPUTER_USE_BACKEND=noop before first call.
}

pub struct ComputerUseTool;

#[async_trait]
impl ToolHandler for ComputerUseTool {
    fn name(&self) -> &'static str {
        "computer_use"
    }

    fn toolset(&self) -> &'static str {
        "computer_use"
    }

    fn emoji(&self) -> &'static str {
        "🖥️"
    }

    fn schema(&self) -> ToolSchema {
        schema::computer_use_schema()
    }

    fn check_fn(&self, ctx: &ToolContext) -> bool {
        if !ctx.config.computer_use_enabled {
            return false;
        }
        if backend_name() == "noop" {
            return true;
        }
        permissions::check_requirements(&ctx.config.computer_use_cua_cmd)
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, ToolError> {
        if !ctx.config.computer_use_enabled {
            return Err(ToolError::PermissionDenied(
                "computer_use is disabled. Set computer_use.enabled: true in config.yaml.".into(),
            ));
        }

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "computer_use".into(),
                message: "missing `action`".into(),
            })?
            .to_ascii_lowercase();

        if action == "type"
            && let Some(text) = args.get("text").and_then(|v| v.as_str())
            && let Some(pat) = safety::blocked_type_pattern(text)
        {
            return Ok(json!({
                "error": format!("blocked pattern in type text: {pat:?}"),
                "hint": "Dangerous shell patterns cannot be typed via computer_use."
            })
            .to_string());
        }
        if action == "key"
            && let Some(keys) = args.get("keys").and_then(|v| v.as_str())
            && let Some(blocked) = safety::blocked_key_combo(keys)
        {
            return Ok(json!({
                "error": format!("blocked key combo: {blocked:?}"),
                "hint": "Destructive system shortcuts are hard-blocked."
            })
            .to_string());
        }

        safety::ensure_destructive_approved(ctx, &action, &args).await?;

        let home = ctx.config.edgecrab_home.clone();
        let cmd = ctx.config.computer_use_cua_cmd.clone();
        let action_owned = action;
        let args_owned = args;

        if BACKEND.get().is_none() {
            let name = backend_name();
            let mut backend: Box<dyn backend::ComputerUseBackend> = if name == "noop" {
                Box::new(noop::NoopBackend::new())
            } else {
                Box::new(cua_backend::CuaDriverBackend::new(&cmd))
            };
            backend.start().await.map_err(|e| ToolError::ExecutionFailed {
                tool: "computer_use".into(),
                message: e,
            })?;
            let _ = BACKEND.set(Mutex::new(backend));
        }
        let mutex = BACKEND.get().ok_or_else(|| ToolError::Other("backend init failed".into()))?;
        let mut guard = mutex.lock().await;
        dispatch::dispatch_action(
            guard.as_mut(),
            &action_owned,
            &args_owned,
            &home,
            Some(ctx),
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "computer_use".into(),
            message: e,
        })
    }
}

inventory::submit!(&ComputerUseTool as &dyn ToolHandler);
