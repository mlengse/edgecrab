//! Tool result spill-to-artifact — shared by conversation loop and web tools.
//!
//! Large tool outputs are written under `cwd/.edgecrab-artifacts/<session_id>/`
//! with a compact preview stub returned to the model.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::config_ref::AppConfigRef;
use crate::registry::ToolContext;

/// Proactive inline cap for `web_extract` body text (before artifact write).
pub const WEB_EXTRACT_INLINE_BYTES: usize = 4_096;

/// Proactive inline cap for `web_search` JSON payload.
pub const WEB_SEARCH_INLINE_BYTES: usize = 8_192;

/// Configuration for tool result spilling.
#[derive(Debug, Clone)]
pub struct SpillConfig {
    /// Whether spilling is enabled (gated by `tools.result_spill`).
    pub enabled: bool,
    /// Byte threshold — results strictly larger than this are spilled (conversation layer).
    pub threshold: usize,
    /// Number of lines to include in the preview stub.
    pub preview_lines: usize,
}

impl Default for SpillConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 16_384,
            preview_lines: 80,
        }
    }
}

impl From<&AppConfigRef> for SpillConfig {
    fn from(cfg: &AppConfigRef) -> Self {
        Self {
            enabled: cfg.result_spill,
            threshold: cfg.result_spill_threshold,
            preview_lines: cfg.result_spill_preview_lines,
        }
    }
}

/// Outcome of a spill attempt.
#[derive(Debug)]
pub enum SpillOutcome {
    /// Result was small enough — use it as-is.
    Inline(String),
    /// Result was spilled to an artifact file.
    Spilled {
        /// The stub message to inject into session.messages.
        stub: String,
        /// Absolute path of the artifact file on disk.
        artifact_path: PathBuf,
        /// Original byte length of the full result.
        original_bytes: usize,
        /// Original line count of the full result.
        original_lines: usize,
        /// Number of preview lines included in the stub.
        preview_line_count: usize,
    },
}

/// Metadata returned when a tool proactively writes an artifact.
#[derive(Debug, Clone)]
pub struct SpillWritten {
    pub rel_path: PathBuf,
    pub abs_path: PathBuf,
    pub preview: String,
    pub preview_line_count: usize,
    pub total_bytes: usize,
    pub total_lines: usize,
}

/// Per-session atomic sequence counter for artifact filenames.
pub struct SpillSequence(AtomicU32);

impl SpillSequence {
    pub fn new() -> Self {
        Self(AtomicU32::new(1))
    }

    pub fn next(&self) -> u32 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for SpillSequence {
    fn default() -> Self {
        Self::new()
    }
}

static FALLBACK_SEQ: OnceLock<Mutex<HashMap<String, AtomicU32>>> = OnceLock::new();

fn next_sequence(session_id: &str, seq: Option<&SpillSequence>) -> u32 {
    if let Some(s) = seq {
        return s.next();
    }
    let map = FALLBACK_SEQ.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().expect("fallback spill seq lock");
    guard
        .entry(session_id.to_string())
        .or_insert_with(|| AtomicU32::new(1))
        .fetch_add(1, Ordering::Relaxed)
}

/// Effective proactive threshold for `web_extract` content bodies.
pub fn web_extract_inline_threshold(config: &SpillConfig) -> usize {
    if !config.enabled {
        return usize::MAX;
    }
    config.threshold.min(WEB_EXTRACT_INLINE_BYTES)
}

/// Effective proactive threshold for `web_search` JSON payloads.
pub fn web_search_inline_threshold(config: &SpillConfig) -> usize {
    if !config.enabled {
        return usize::MAX;
    }
    config.threshold.min(WEB_SEARCH_INLINE_BYTES)
}

/// Attempt to spill a tool result to an artifact file (conversation post-dispatch).
pub fn maybe_spill(
    tool_name: &str,
    _tool_call_id: &str,
    result: String,
    session_id: &str,
    cwd: &Path,
    config: &SpillConfig,
    seq: &SpillSequence,
) -> SpillOutcome {
    if !config.enabled {
        return SpillOutcome::Inline(result);
    }

    if result.len() <= config.threshold {
        return SpillOutcome::Inline(result);
    }

    if tool_name == "computer_use" {
        return SpillOutcome::Inline(result);
    }

    match write_artifact_proactive(
        tool_name,
        &result,
        session_id,
        cwd,
        config,
        Some(seq),
    ) {
        Some(written) => {
            let stub = build_conversation_stub(
                tool_name,
                &result,
                &written.rel_path,
                written.total_bytes,
                written.total_lines,
                &written.preview,
                written.preview_line_count,
            );
            tracing::info!(
                tool = %tool_name,
                original_bytes = written.total_bytes,
                original_lines = written.total_lines,
                preview_lines = written.preview_line_count,
                artifact = %written.rel_path.display(),
                "tool result spilled to artifact"
            );
            SpillOutcome::Spilled {
                stub,
                artifact_path: written.abs_path,
                original_bytes: written.total_bytes,
                original_lines: written.total_lines,
                preview_line_count: written.preview_line_count,
            }
        }
        None => SpillOutcome::Inline(result),
    }
}

/// Proactively write full content to disk (caller already decided size warrants spill).
pub fn write_artifact_proactive(
    tool_name: &str,
    body: &str,
    session_id: &str,
    cwd: &Path,
    config: &SpillConfig,
    seq: Option<&SpillSequence>,
) -> Option<SpillWritten> {
    if !config.enabled {
        return None;
    }

    let safe_name = sanitize_tool_name(tool_name);
    let seq_num = next_sequence(session_id, seq);
    let artifact_dir = artifact_dir_for_session(cwd, session_id);
    let filename = format!("{safe_name}_{seq_num:03}.md");
    let artifact_path = artifact_dir.join(&filename);

    if write_artifact(&artifact_dir, &artifact_path, body).is_err() {
        tracing::warn!(
            tool = %tool_name,
            path = %artifact_path.display(),
            "proactive artifact write failed"
        );
        return None;
    }

    ensure_gitignore(cwd);

    let total_bytes = body.len();
    let total_lines = body.lines().count().max(1);
    let (preview, preview_line_count) = build_preview(body, config.preview_lines);
    let rel_path = artifact_path.strip_prefix(cwd).unwrap_or(&artifact_path).to_path_buf();

    Some(SpillWritten {
        rel_path,
        abs_path: artifact_path,
        preview,
        preview_line_count,
        total_bytes,
        total_lines,
    })
}

/// Hermes layer-3: after all tools in one assistant turn, spill largest results until
/// aggregate tool payload is under `turn_budget_chars`. `computer_use` is never spilled.
pub fn enforce_turn_budget(
    messages: &mut [edgecrab_types::Message],
    turn_budget_chars: usize,
    spill_config: &SpillConfig,
    session_id: &str,
    cwd: &Path,
    seq: &SpillSequence,
) -> usize {
    use edgecrab_types::Role;

    if turn_budget_chars == 0 || !spill_config.enabled {
        return 0;
    }

    let mut total: usize = 0;
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for (i, msg) in messages.iter().enumerate() {
        if msg.role != Role::Tool {
            continue;
        }
        let body = msg.text_content();
        let size = body.len();
        total += size;
        if msg.name.as_deref() == Some("computer_use") {
            continue;
        }
        if body.contains("[tool_result_spill]") {
            continue;
        }
        candidates.push((i, size));
    }

    if total <= turn_budget_chars {
        return 0;
    }

    candidates.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    let mut spilled = 0usize;
    for (idx, size) in candidates {
        if total <= turn_budget_chars {
            break;
        }
        let tool_name = messages[idx].name.as_deref().unwrap_or("tool");
        let tool_call_id = messages[idx].tool_call_id.as_deref().unwrap_or("budget");
        let body = messages[idx].text_content();
        match maybe_spill(
            tool_name,
            tool_call_id,
            body,
            session_id,
            cwd,
            spill_config,
            seq,
        ) {
            SpillOutcome::Spilled { stub, .. } => {
                total = total.saturating_sub(size).saturating_add(stub.len());
                messages[idx] =
                    edgecrab_types::Message::tool_result(tool_call_id, tool_name, &stub);
                spilled += 1;
            }
            SpillOutcome::Inline(_) => {}
        }
    }
    spilled
}

/// Build a compact JSON stub for a spilled `web_search` payload.
pub fn web_search_spilled_json(
    query: &str,
    backend: &str,
    fallback_from: Option<&str>,
    note: Option<&str>,
    results: &[crate::tools::web::search::backend::SearchResult],
    written: &SpillWritten,
) -> serde_json::Value {
    let preview_rows: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "title": r.title,
                "url": r.url,
                "position": r.rank,
            })
        })
        .collect();

    serde_json::json!({
        "success": true,
        "query": query,
        "backend": backend,
        "fallback_from": fallback_from,
        "note": note,
        "result_spilled": true,
        "result_count": results.len(),
        "artifact": written.rel_path,
        "content_bytes": written.total_bytes,
        "preview_lines": written.preview_line_count,
        "results_preview": preview_rows,
        "hint": format!(
            "Full search JSON ({} bytes) saved to {}. Use read_file for snippets and metadata.",
            written.total_bytes,
            written.rel_path.display()
        ),
    })
}

/// Apply proactive content spill to a web extract document value.
pub fn apply_web_extract_content_spill(
    mut doc: serde_json::Value,
    ctx: &ToolContext,
    seq: Option<&SpillSequence>,
) -> serde_json::Value {
    let config = SpillConfig::from(&ctx.config);
    let threshold = web_extract_inline_threshold(&config);
    let content = doc
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if content.len() <= threshold {
        return doc;
    }

    let content_bytes = content.len();
    let title = doc
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("(untitled)");
    let url = doc.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let extractor = doc
        .get("extractor")
        .and_then(|v| v.as_str())
        .unwrap_or("native");

    let artifact_body = format!(
        "# {title}\n\nURL: {url}\nExtractor: {extractor}\nBytes: {}\n\n---\n\n{content}",
        content.len()
    );

    let Some(written) = write_artifact_proactive(
        "web_extract",
        &artifact_body,
        &ctx.session_id,
        &ctx.cwd,
        &config,
        seq,
    ) else {
        return doc;
    };

    if let Some(obj) = doc.as_object_mut() {
        obj.insert("content".into(), serde_json::Value::String(written.preview.clone()));
        obj.insert("content_spilled".into(), serde_json::Value::Bool(true));
        obj.insert(
            "content_bytes".into(),
            serde_json::Value::Number(content_bytes.into()),
        );
        obj.insert(
            "artifact".into(),
            serde_json::Value::String(written.rel_path.display().to_string()),
        );
        obj.insert(
            "preview_lines".into(),
            serde_json::Value::Number(written.preview_line_count.into()),
        );
    }

    doc
}

fn build_conversation_stub(
    tool_name: &str,
    _full_result: &str,
    rel_path: &Path,
    original_bytes: usize,
    original_lines: usize,
    preview: &str,
    preview_line_count: usize,
) -> String {
    let pct = if original_lines > 0 {
        (preview_line_count as f64 / original_lines as f64 * 100.0).round() as usize
    } else {
        100
    };

    format!(
        "[tool_result_spill]\n\
         tool: {tool_name}\n\
         lines: {original_lines}\n\
         bytes: {original_bytes}\n\
         artifact: {rel}\n\
         showing: {preview_line_count}/{original_lines} lines (first {pct}%)\n\
         \n\
         --- BEGIN PREVIEW ({preview_line_count} lines) ---\n\
         {preview}\n\
         --- END PREVIEW ---\n\
         \n\
         Full result saved to: {rel}\n\
         Use read_file or file_search to explore the full content.",
        rel = rel_path.display(),
    )
}

fn sanitize_tool_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn artifact_dir_for_session(cwd: &Path, session_id: &str) -> PathBuf {
    let safe_session: String = session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cwd.join(".edgecrab-artifacts").join(safe_session)
}

fn write_artifact(dir: &Path, path: &Path, content: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn build_preview(result: &str, max_lines: usize) -> (String, usize) {
    let sanitized = sanitize_preview_source(result);
    let preview = truncate_preview_chars(&sanitized, max_lines, MAX_PREVIEW_CHARS);
    let count = preview.lines().count().max(1);
    (preview, count)
}

const MAX_PREVIEW_CHARS: usize = 2_048;
const MAX_PREVIEW_LINE_CHARS: usize = 512;

fn sanitize_preview_source(result: &str) -> String {
    if let Some(summary) = edgecrab_types::multimodal_text_summary(result) {
        return summary;
    }
    redact_data_urls(result)
}

fn redact_data_urls(text: &str) -> String {
    const PREFIXES: &[&str] = &[
        "data:image/png;base64,",
        "data:image/jpeg;base64,",
        "data:image/jpg;base64,",
        "data:image/webp;base64,",
    ];
    let mut out = text.to_string();
    for prefix in PREFIXES {
        while let Some(start) = out.find(prefix) {
            let rest = &out[start + prefix.len()..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '}')
                .unwrap_or(rest.len());
            let redacted = format!("[image data: {end} bytes redacted from preview]");
            out.replace_range(start..start + prefix.len() + end, &redacted);
        }
    }
    out
}

fn truncate_preview_chars(text: &str, max_lines: usize, max_chars: usize) -> String {
    let mut lines = Vec::new();
    let mut total = 0usize;
    for line in text.lines().take(max_lines) {
        let clipped = if line.chars().count() > MAX_PREVIEW_LINE_CHARS {
            let mut s: String = line.chars().take(MAX_PREVIEW_LINE_CHARS).collect();
            s.push('…');
            s
        } else {
            line.to_string()
        };
        total += clipped.len() + 1;
        if total > max_chars {
            lines.push("… [preview truncated — read artifact for full output]".into());
            break;
        }
        lines.push(clipped);
    }
    if lines.is_empty() {
        return "[empty tool result]".into();
    }
    lines.join("\n")
}

fn ensure_gitignore(cwd: &Path) {
    let gitignore_path = cwd.join(".gitignore");
    let entry = ".edgecrab-artifacts/";

    let existing = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
    if existing.lines().any(|line| line.trim() == entry) {
        return;
    }

    let addition = if existing.is_empty() || existing.ends_with('\n') {
        format!("{entry}\n")
    } else {
        format!("\n{entry}\n")
    };

    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, addition.as_bytes()))
    {
        tracing::debug!(error = %e, "could not update .gitignore with artifact entry");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(enabled: bool, threshold: usize, preview_lines: usize) -> SpillConfig {
        SpillConfig {
            enabled,
            threshold,
            preview_lines,
        }
    }

    #[test]
    fn inline_when_disabled() {
        let tmp = TempDir::new().expect("tempdir");
        let seq = SpillSequence::new();
        let config = test_config(false, 10, 5);
        let result = "a".repeat(1000);

        match maybe_spill(
            "test",
            "tc1",
            result.clone(),
            "ses1",
            tmp.path(),
            &config,
            &seq,
        ) {
            SpillOutcome::Inline(s) => assert_eq!(s, result),
            SpillOutcome::Spilled { .. } => panic!("should not spill when disabled"),
        }
    }

    #[test]
    fn spills_when_over_threshold() {
        let tmp = TempDir::new().expect("tempdir");
        let seq = SpillSequence::new();
        let config = test_config(true, 100, 5);
        let lines: Vec<String> = (1..=50).map(|i| format!("line {i}")).collect();
        let result = lines.join("\n");

        match maybe_spill(
            "file_search",
            "tc1",
            result.clone(),
            "ses1",
            tmp.path(),
            &config,
            &seq,
        ) {
            SpillOutcome::Spilled {
                stub,
                artifact_path,
                original_bytes,
                original_lines,
                preview_line_count,
            } => {
                assert_eq!(original_bytes, result.len());
                assert_eq!(original_lines, 50);
                assert_eq!(preview_line_count, 5);
                assert!(stub.contains("[tool_result_spill]"));
                assert!(artifact_path.exists());
                let written = std::fs::read_to_string(&artifact_path).expect("read artifact");
                assert_eq!(written, result);
            }
            SpillOutcome::Inline(_) => panic!("should spill over threshold"),
        }
    }

    #[test]
    fn web_extract_threshold_is_lower_than_global() {
        let config = test_config(true, 16_384, 80);
        assert_eq!(web_extract_inline_threshold(&config), WEB_EXTRACT_INLINE_BYTES);
        assert_eq!(web_search_inline_threshold(&config), WEB_SEARCH_INLINE_BYTES);
    }

    #[test]
    fn proactive_web_extract_spill_replaces_content_with_preview() {
        let tmp = TempDir::new().expect("tempdir");
        let mut ctx = ToolContext::test_context();
        ctx.cwd = tmp.path().to_path_buf();
        ctx.session_id = "web-extract-test".into();
        ctx.config.result_spill = true;

        let big = "word ".repeat(2_000);
        let doc = serde_json::json!({
            "url": "https://example.com/page",
            "title": "Example",
            "content": big,
            "extractor": "native",
        });

        let out = apply_web_extract_content_spill(doc, &ctx, None);
        assert_eq!(out["content_spilled"], true);
        assert!(out["artifact"].as_str().is_some_and(|p| p.contains("web_extract")));
        assert!(out["content"].as_str().is_some_and(|c| c.len() < big.len()));
        assert_eq!(out["content_bytes"].as_u64(), Some(big.len() as u64));
    }

    #[test]
    fn computer_use_never_spills() {
        let tmp = TempDir::new().expect("tempdir");
        let seq = SpillSequence::new();
        let config = test_config(true, 100, 5);
        let result = "x".repeat(8_000);
        match maybe_spill(
            "computer_use",
            "tc1",
            result.clone(),
            "ses1",
            tmp.path(),
            &config,
            &seq,
        ) {
            SpillOutcome::Inline(s) => assert_eq!(s, result),
            SpillOutcome::Spilled { .. } => panic!("computer_use must not spill"),
        }
    }

    #[test]
    fn proactive_web_search_spill_emits_compact_stub() {
        use crate::tools::web::search::backend::SearchResult;

        let tmp = TempDir::new().expect("tempdir");
        let big_snippet = "snippet ".repeat(500);
        let results: Vec<SearchResult> = (0..20)
            .map(|i| {
                SearchResult::new(
                    i + 1,
                    format!("Result {i}"),
                    format!("https://example.com/{i}"),
                    big_snippet.clone(),
                    "ddgs",
                )
            })
            .collect();
        let payload = crate::tools::web::search::response::success_payload(
            "test query",
            "ddgs",
            None,
            None,
            &results,
        );
        let json_str = payload.to_string();
        assert!(json_str.len() > WEB_SEARCH_INLINE_BYTES);

        let config = test_config(true, 16_384, 10);
        let written = write_artifact_proactive(
            "web_search",
            &json_str,
            "ses-search",
            tmp.path(),
            &config,
            None,
        )
        .expect("should write artifact");

        let stub = web_search_spilled_json(
            "test query",
            "ddgs",
            None,
            None,
            &results,
            &written,
        );
        assert_eq!(stub["result_spilled"], true);
        assert!(stub["artifact"].as_str().is_some());
        assert_eq!(stub["result_count"], 20);
        assert!(stub.to_string().len() < json_str.len() / 2);
    }
}
