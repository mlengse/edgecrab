//! Post-write LSP diagnostic gate — trait boundary between tools and `edgecrab-lsp`.
//!
//! WHY the trait lives here: `edgecrab-lsp` already depends on `edgecrab-tools`.
//! Tools must not depend on the LSP crate directly; the conversation loop injects
//! `Arc<dyn LspGate>` at runtime when `lsp.enabled` is true.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};

/// Pre/post content for line-shift aware delta filtering (Hermes parity).
#[derive(Debug, Clone, Copy, Default)]
pub struct LspEditContext<'a> {
    pub pre_content: Option<&'a str>,
    pub post_content: Option<&'a str>,
}

/// Severity-normalized diagnostic attached to a successful file-mutation tool result.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolDiagnostic {
    pub severity: String,
    pub line: u32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Fetches semantic diagnostics for a path after a successful write/patch.
#[async_trait]
pub trait LspGate: Send + Sync {
    /// Snapshot diagnostics before a write (Hermes `snapshot_baseline`).
    async fn snapshot_baseline(&self, ctx: &crate::registry::ToolContext, path: &Path, timeout: Duration);

    /// Pull diagnostics introduced since the last baseline (delta + optional line shift).
    async fn pull_diagnostics(
        &self,
        ctx: &crate::registry::ToolContext,
        path: &Path,
        timeout: Duration,
        edit: Option<LspEditContext<'_>>,
    ) -> Vec<ToolDiagnostic>;
}

const MAX_PER_FILE: usize = 20;
const MAX_TOTAL_CHARS: usize = 4_000;

/// Hermes-compatible `<diagnostics>` block with truncation.
pub fn format_lsp_diagnostics_block(path: &Path, items: &[ToolDiagnostic]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let limited = items.iter().take(MAX_PER_FILE);
    let extra = items.len().saturating_sub(MAX_PER_FILE);
    let mut lines: Vec<String> = limited
        .map(|item| {
            let code = item
                .code
                .as_ref()
                .map(|c| format!(" [{c}]"))
                .unwrap_or_default();
            format!(
                "{} [{}:{}] {}{}",
                item.severity.to_uppercase(),
                item.line,
                1,
                item.message,
                code
            )
        })
        .collect();
    if extra > 0 {
        lines.push(format!("... and {extra} more"));
    }
    let body = lines.join("\n");
    let block = format!(
        "<diagnostics file=\"{}\">\n{body}\n</diagnostics>",
        path.display()
    );
    let prefix = "LSP diagnostics introduced by this edit:\n";
    Some(truncate_lsp_block(&format!("{prefix}{block}")))
}

fn truncate_lsp_block(s: &str) -> String {
    if s.len() <= MAX_TOTAL_CHARS {
        return s.to_string();
    }
    const MARKER: &str = "\n…[truncated]";
    let keep = MAX_TOTAL_CHARS.saturating_sub(MARKER.len());
    format!("{}{MARKER}", &s[..keep])
}

/// Captures pre-edit content and LSP baseline before a file mutation.
pub struct LspWriteHook {
    pre_content: Option<String>,
}

impl LspWriteHook {
    /// Hook with known pre-edit content (e.g. apply_patch backups); does not re-snapshot.
    pub fn with_pre_content(pre_content: Option<String>) -> Self {
        Self { pre_content }
    }

    /// Read pre-edit content (best-effort) and snapshot LSP baseline.
    pub async fn capture_before(ctx: &crate::registry::ToolContext, path: &Path) -> Self {
        let pre_content = tokio::fs::read_to_string(path).await.ok();
        if ctx.config.lsp_enabled
            && let Some(gate) = ctx.lsp_gate.as_ref()
        {
            let timeout = Duration::from_millis(ctx.config.lsp_post_write_timeout_ms.max(1));
            gate.snapshot_baseline(ctx, path, timeout).await;
        }
        Self { pre_content }
    }

    /// Attach post-write diagnostics to a JSON tool-success object.
    pub async fn attach_after(
        self,
        ctx: &crate::registry::ToolContext,
        path: &Path,
        result: &mut Value,
        post_content: &str,
    ) {
        attach_post_write_diagnostics(
            ctx,
            path,
            result,
            self.pre_content.as_deref(),
            Some(post_content),
        )
        .await;
    }
}

/// Attach post-write diagnostics to a JSON tool-success object (best-effort).
pub async fn attach_post_write_diagnostics(
    ctx: &crate::registry::ToolContext,
    path: &Path,
    result: &mut Value,
    pre_content: Option<&str>,
    post_content: Option<&str>,
) {
    if !ctx.config.lsp_enabled {
        return;
    }
    let Some(gate) = ctx.lsp_gate.as_ref() else {
        return;
    };
    let timeout_ms = ctx.config.lsp_post_write_timeout_ms.max(1);
    let timeout = Duration::from_millis(timeout_ms);
    let edit = Some(LspEditContext {
        pre_content,
        post_content,
    });
    let diagnostics = match tokio::time::timeout(
        timeout,
        gate.pull_diagnostics(ctx, path, timeout, edit),
    )
    .await
    {
        Ok(items) => items,
        Err(_) => {
            tracing::debug!(
                path = %path.display(),
                timeout_ms,
                "post-write LSP diagnostics timed out"
            );
            Vec::new()
        }
    };
    if let Some(obj) = result.as_object_mut() {
        obj.insert("diagnostics".into(), json!(diagnostics));
        if let Some(block) = format_lsp_diagnostics_block(path, &diagnostics) {
            obj.insert("lsp_diagnostics".into(), json!(block));
        }
    }
}

#[cfg(test)]
pub struct MockLspGate {
    pub diagnostics: Vec<ToolDiagnostic>,
}

#[cfg(test)]
#[async_trait]
impl LspGate for MockLspGate {
    async fn snapshot_baseline(
        &self,
        _ctx: &crate::registry::ToolContext,
        _path: &Path,
        _timeout: Duration,
    ) {
    }

    async fn pull_diagnostics(
        &self,
        _ctx: &crate::registry::ToolContext,
        _path: &Path,
        _timeout: Duration,
        _edit: Option<LspEditContext<'_>>,
    ) -> Vec<ToolDiagnostic> {
        self.diagnostics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolContext;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn attach_injects_diagnostics_array() {
        let dir = TempDir::new().expect("tmpdir");
        let mut ctx = ToolContext::test_context();
        ctx.cwd = dir.path().to_path_buf();
        ctx.config.lsp_enabled = true;
        ctx.lsp_gate = Some(Arc::new(MockLspGate {
            diagnostics: vec![ToolDiagnostic {
                severity: "error".into(),
                line: 1,
                message: "cannot find type `Bar`".into(),
                code: Some("E0412".into()),
            }],
        }));

        let mut value = json!({"ok": true, "path": "src/foo.rs"});
        attach_post_write_diagnostics(&ctx, dir.path().join("src/foo.rs").as_path(), &mut value, None, None)
            .await;

        let diags = value["diagnostics"].as_array().expect("diagnostics array");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0]["severity"], "error");
        assert!(value["lsp_diagnostics"]
            .as_str()
            .unwrap()
            .contains("<diagnostics"));
    }

    #[tokio::test]
    async fn attach_noop_when_lsp_disabled() {
        let dir = TempDir::new().expect("tmpdir");
        let mut ctx = ToolContext::test_context();
        ctx.config.lsp_enabled = false;
        ctx.lsp_gate = Some(Arc::new(MockLspGate {
            diagnostics: vec![ToolDiagnostic {
                severity: "error".into(),
                line: 1,
                message: "x".into(),
                code: None,
            }],
        }));

        let mut value = json!({"ok": true});
        attach_post_write_diagnostics(&ctx, dir.path().join("x.rs").as_path(), &mut value, None, None).await;
        assert!(value.get("diagnostics").is_none());
    }

    #[tokio::test]
    async fn write_hook_skips_attach_when_lsp_disabled() {
        let dir = TempDir::new().expect("tmpdir");
        let path = dir.path().join("main.rs");
        tokio::fs::write(&path, "fn main() {}\n").await.expect("write");
        let mut ctx = ToolContext::test_context();
        ctx.cwd = dir.path().to_path_buf();
        ctx.config.lsp_enabled = false;
        ctx.lsp_gate = Some(Arc::new(MockLspGate {
            diagnostics: vec![ToolDiagnostic {
                severity: "error".into(),
                line: 1,
                message: "should not appear".into(),
                code: None,
            }],
        }));

        let hook = LspWriteHook::capture_before(&ctx, &path).await;
        let mut value = json!({"ok": true});
        hook.attach_after(&ctx, &path, &mut value, "fn main() {}\n").await;
        assert!(value.get("diagnostics").is_none());
    }

    #[test]
    fn truncate_caps_long_blocks() {
        let long = "x".repeat(5000);
        let out = truncate_lsp_block(&long);
        assert!(out.len() <= MAX_TOTAL_CHARS);
        assert!(out.ends_with("…[truncated]"));
    }
}
