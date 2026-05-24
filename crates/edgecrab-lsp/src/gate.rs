//! Post-write diagnostic gate — wires `edgecrab-lsp` into file mutation tools.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use edgecrab_tools::lsp_gate::{LspEditContext, LspGate, ToolDiagnostic};
use edgecrab_tools::registry::ToolContext;
use edgecrab_tools::tools::backends::BackendKind;
use edgecrab_tools::path_utils::jail_read_path;
use lsp_types::request::DocumentDiagnosticRequest;
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, PartialResultParams, TextDocumentIdentifier,
    WorkDoneProgressParams,
};
use tokio::time::sleep;

use crate::delta::filter_introduced_diagnostics;
use crate::error::{LspError, path_to_uri};
use crate::manager::runtime_for_ctx;
use crate::range_shift::build_line_shift;

static WARNED_NO_SERVER: AtomicBool = AtomicBool::new(false);

pub struct EdgecrabLspGate;

fn resolve_mutation_path(ctx: &ToolContext, path: &Path) -> Result<PathBuf, LspError> {
    let policy = ctx.config.file_path_policy(&ctx.cwd);
    let relative = path
        .strip_prefix(&ctx.cwd)
        .unwrap_or(path)
        .to_string_lossy();
    if let Ok(resolved) = jail_read_path(&relative, &policy) {
        return Ok(resolved);
    }
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    jail_read_path(&canonical.to_string_lossy(), &policy)
        .map_err(|err| LspError::Other(err.to_string()))
}

fn baseline_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn text_document(uri: lsp_types::Uri) -> TextDocumentIdentifier {
    TextDocumentIdentifier { uri }
}

fn diagnostic_to_tool(diagnostic: &Diagnostic) -> Option<ToolDiagnostic> {
    let severity = diagnostic.severity?;
    let severity = match severity {
        DiagnosticSeverity::ERROR => "error",
        DiagnosticSeverity::WARNING => "warning",
        _ => return None,
    };
    Some(ToolDiagnostic {
        severity: severity.to_string(),
        line: diagnostic.range.start.line.saturating_add(1),
        message: diagnostic.message.clone(),
        code: diagnostic.code.as_ref().map(|code| match code {
            lsp_types::NumberOrString::Number(n) => n.to_string(),
            lsp_types::NumberOrString::String(s) => s.clone(),
        }),
    })
}

fn filter_tool_diagnostics(items: Vec<Diagnostic>) -> Vec<ToolDiagnostic> {
    items
        .iter()
        .filter_map(diagnostic_to_tool)
        .collect()
}

async fn wait_for_push_diagnostics(
    cache: &crate::diagnostics::DiagnosticCache,
    uri: &lsp_types::Uri,
    budget: Duration,
) -> Vec<Diagnostic> {
    let deadline = Instant::now() + budget;
    loop {
        if let Some(items) = cache.get(uri)
            && !items.is_empty()
        {
            return items;
        }
        if Instant::now() >= deadline {
            break;
        }
        sleep(Duration::from_millis(25)).await;
    }
    cache.get(uri).unwrap_or_default()
}

async fn pull_raw_diagnostics(ctx: &ToolContext, path: &Path) -> Result<Vec<Diagnostic>, LspError> {
    let runtime = runtime_for_ctx(ctx)?;
    let path = resolve_mutation_path(ctx, path)?;
    let server = runtime.manager.server_for_file(&path).await?;
    let guard = runtime
        .sync
        .ensure_open(
            server.connection.clone(),
            &path,
            &server.language_id,
            ctx.config.lsp_file_size_limit_bytes,
        )
        .await?;
    runtime
        .sync
        .refresh_from_disk(
            &server.connection,
            &path,
            ctx.config.lsp_file_size_limit_bytes,
        )
        .await?;
    let uri = path_to_uri(&path)?;

    let diagnostics = if server.capabilities.diagnostic_provider.is_some() {
        let report: DocumentDiagnosticReportResult = server
            .connection
            .request::<DocumentDiagnosticRequest>(DocumentDiagnosticParams {
                text_document: text_document(uri.clone()),
                identifier: None,
                previous_result_id: None,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await?;
        match report {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
                full.full_document_diagnostic_report.items
            }
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Unchanged(_)) => {
                Vec::new()
            }
            DocumentDiagnosticReportResult::Partial(_) => Vec::new(),
        }
    } else {
        wait_for_push_diagnostics(&runtime.diagnostics, &uri, Duration::from_millis(150)).await
    };

    drop(guard);
    Ok(diagnostics)
}

async fn pull_with_delta(
    ctx: &ToolContext,
    path: &Path,
    edit: Option<LspEditContext<'_>>,
) -> Result<Vec<ToolDiagnostic>, LspError> {
    let runtime = runtime_for_ctx(ctx)?;
    let resolved = resolve_mutation_path(ctx, path)?;
    let key = baseline_key(&resolved);

    let current = pull_raw_diagnostics(ctx, path).await?;
    let baseline = runtime
        .delta_baselines
        .get(&key)
        .map(|entry| entry.value().clone())
        .unwrap_or_default();

    let line_shift = edit.and_then(|e| {
        let pre = e.pre_content?;
        let post = e.post_content?;
        if pre == post {
            None
        } else {
            Some(build_line_shift(pre, post))
        }
    });

    let introduced = filter_introduced_diagnostics(current.clone(), &baseline, line_shift.as_ref());

    // Roll baseline forward (Hermes diagnosticTracking).
    runtime.delta_baselines.insert(key, current);

    Ok(filter_tool_diagnostics(introduced))
}

fn gate_unavailable(_ctx: &ToolContext, path: &Path, err: LspError) -> Vec<ToolDiagnostic> {
    if matches!(err, LspError::NoServerForFile { .. } | LspError::Disabled) {
        if !WARNED_NO_SERVER.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                error = %err,
                path = %path.display(),
                "post-write LSP diagnostics unavailable for this file type"
            );
        }
    } else {
        tracing::debug!(error = %err, path = %path.display(), "post-write LSP pull failed");
    }
    Vec::new()
}

#[async_trait]
impl LspGate for EdgecrabLspGate {
    async fn snapshot_baseline(&self, ctx: &ToolContext, path: &Path, timeout: Duration) {
        if !ctx.config.lsp_enabled || !matches!(ctx.config.terminal_backend, BackendKind::Local) {
            return;
        }
        let result = tokio::time::timeout(timeout, pull_raw_diagnostics(ctx, path)).await;
        let Ok(Ok(items)) = result else {
            return;
        };
        if let Ok(runtime) = runtime_for_ctx(ctx)
            && let Ok(resolved) = resolve_mutation_path(ctx, path)
        {
            runtime
                .delta_baselines
                .insert(baseline_key(&resolved), items);
        }
    }

    async fn pull_diagnostics(
        &self,
        ctx: &ToolContext,
        path: &Path,
        timeout: Duration,
        edit: Option<LspEditContext<'_>>,
    ) -> Vec<ToolDiagnostic> {
        if !ctx.config.lsp_enabled {
            return Vec::new();
        }
        if !matches!(ctx.config.terminal_backend, BackendKind::Local) {
            return Vec::new();
        }

        match tokio::time::timeout(timeout, pull_with_delta(ctx, path, edit)).await {
            Ok(Ok(items)) => items,
            Ok(Err(err)) => gate_unavailable(ctx, path, err),
            Err(_) => {
                tracing::debug!(
                    path = %path.display(),
                    "post-write LSP diagnostics timed out"
                );
                Vec::new()
            }
        }
    }
}
