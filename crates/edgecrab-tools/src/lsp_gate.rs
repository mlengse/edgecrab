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
    async fn pull_diagnostics(
        &self,
        ctx: &crate::registry::ToolContext,
        path: &Path,
        timeout: Duration,
    ) -> Vec<ToolDiagnostic>;
}

/// Hermes-compatible formatted block for models that scan prose tool results.
pub fn format_lsp_diagnostics_block(path: &Path, items: &[ToolDiagnostic]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let mut lines = vec![format!(
        "LSP diagnostics introduced by this edit ({})",
        path.display()
    )];
    for item in items {
        lines.push(format!(
            "{} [{}:{}] {}",
            item.severity.to_uppercase(),
            path.display(),
            item.line,
            item.message
        ));
    }
    Some(lines.join("\n"))
}

/// Attach post-write diagnostics to a JSON tool-success object (best-effort).
pub async fn attach_post_write_diagnostics(
    ctx: &crate::registry::ToolContext,
    path: &Path,
    result: &mut Value,
) {
    if !ctx.config.lsp_enabled {
        return;
    }
    let Some(gate) = ctx.lsp_gate.as_ref() else {
        return;
    };
    let timeout_ms = ctx.config.lsp_post_write_timeout_ms.max(1);
    let timeout = Duration::from_millis(timeout_ms);
    let diagnostics =
        match tokio::time::timeout(timeout, gate.pull_diagnostics(ctx, path, timeout)).await
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
    async fn pull_diagnostics(
        &self,
        _ctx: &crate::registry::ToolContext,
        _path: &Path,
        _timeout: Duration,
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
        attach_post_write_diagnostics(&ctx, dir.path().join("src/foo.rs").as_path(), &mut value)
            .await;

        let diags = value["diagnostics"].as_array().expect("diagnostics array");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0]["severity"], "error");
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
        attach_post_write_diagnostics(&ctx, dir.path().join("x.rs").as_path(), &mut value).await;
        assert!(value.get("diagnostics").is_none());
    }
}
