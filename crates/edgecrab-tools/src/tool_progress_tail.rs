//! Throttled tail-line progress for long-running tools (terminal, execute_code, processes).
//!
//! Single source of truth for output sanitization, tail formatting, and throttled
//! `ToolProgressUpdate` emissions (≤5/sec, last N lines).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use strip_ansi_escapes::strip_str;

use crate::registry::{ToolContext, ToolProgressUpdate};

/// Minimum interval between progress emissions (≤5/sec).
pub const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(200);
/// Number of trailing lines included in each progress message.
pub const OUTPUT_TAIL_LINE_COUNT: usize = 3;
/// Heartbeat interval for long-blocking process wait tools.
pub const HEARTBEAT_INTERVAL_SECS: u64 = 2;

pub type OutputProgressFn = Arc<dyn Fn(&str) + Send + Sync>;

/// Optional progress hook passed into terminal backends.
#[derive(Clone, Default)]
pub struct ExecuteOptions {
    pub on_output_line: Option<OutputProgressFn>,
}

// ─── Shared tail formatting (DRY) ─────────────────────────────────────

/// Strip ANSI and trim one output line for display / progress.
pub fn sanitize_output_line(raw: &str) -> String {
    strip_str(raw).trim().to_string()
}

/// Last [`OUTPUT_TAIL_LINE_COUNT`] non-empty lines from a slice.
pub fn format_tail_from_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let start = lines.len().saturating_sub(OUTPUT_TAIL_LINE_COUNT);
    lines[start..].join("\n")
}

/// Last [`OUTPUT_TAIL_LINE_COUNT`] non-empty lines from free text.
pub fn format_tail_from_text(text: &str) -> String {
    let lines: Vec<String> = text
        .lines()
        .map(sanitize_output_line)
        .filter(|line| !line.is_empty())
        .collect();
    format_tail_from_lines(&lines)
}

/// Format a wait-for-process heartbeat from a full output snapshot.
pub fn format_wait_heartbeat(process_id: &str, output: &str) -> String {
    let preview = {
        let trimmed = format_tail_from_text(output);
        if trimmed.is_empty() {
            "(no output yet)".to_string()
        } else {
            trimmed
        }
    };
    format!("still running… [{process_id}]\n{preview}")
}

/// Prefix stderr lines in live progress (shared by sinks and capture paths).
pub fn format_stderr_progress_line(line: &str) -> String {
    format!("[stderr] {line}")
}

/// Verbose-off TUI indicator: `⏳ tool  preview (Ns)` + optional tail lines.
pub fn format_minimal_tool_indicator(
    tool_name: &str,
    preview: &str,
    elapsed_secs: Option<u64>,
    tail: &str,
) -> String {
    let elapsed_suffix = elapsed_secs
        .filter(|secs| *secs > 0)
        .map(|secs| format!(" ({secs}s)"))
        .unwrap_or_default();
    let headline = format!("⏳ {tool_name}  {preview}{elapsed_suffix}");
    if tail.trim().is_empty() {
        headline
    } else {
        format!("{headline}\n{tail}")
    }
}

/// Short milestone for long-running non-shell tools (web search, fetch, crawl).
pub fn format_search_milestone(query: &str) -> String {
    format!("searching `{}`…", truncate_command_preview(query, 72))
}

pub fn format_fetch_milestone(url: &str) -> String {
    format!("fetching {}…", truncate_command_preview(url, 72))
}

pub fn format_backend_attempt_milestone(backend: &str, action: &str) -> String {
    format!("{action} ({backend})…")
}

pub fn format_results_milestone(count: usize, backend: &str) -> String {
    format!("{count} result(s) via {backend}")
}

pub fn format_crawl_page_milestone(index: usize, max_pages: usize, url: &str) -> String {
    format!(
        "crawling page {index}/{max_pages} · {}",
        truncate_command_preview(url, 56)
    )
}

pub fn format_browser_milestone(action: &str, detail: &str) -> String {
    let detail = truncate_command_preview(detail.trim(), 64);
    if detail.is_empty() {
        format!("browser: {action}…")
    } else {
        format!("browser: {action} · {detail}")
    }
}

pub fn format_browser_wait_milestone(label: &str, elapsed_secs: u64) -> String {
    format!(
        "browser: waiting · {} ({elapsed_secs}s)",
        truncate_command_preview(label, 48)
    )
}

// ─── Activity notices (compression, approval — shared TUI + gateway) ───

pub fn format_compression_started() -> String {
    "🗜 Compressing conversation context…".into()
}

pub fn format_compression_done(message_count: usize) -> String {
    format!("🗜 Context compressed ({message_count} messages retained).")
}

pub fn format_compression_circuit_breaker(failures: u32) -> String {
    format!(
        "🗜 Using fast structural compression (LLM summarizer unavailable after {failures} failure(s))…"
    )
}

pub fn format_approval_waiting(command: &str) -> String {
    format!(
        "⏸ Waiting for your approval: `{}`",
        truncate_command_preview(command, 72)
    )
}

/// Send one throttled progress message when the tool invocation has a progress channel.
pub fn emit_tool_progress(ctx: &ToolContext, message: &str) {
    try_send_tool_progress(
        ctx.tool_progress_tx.as_ref(),
        ctx.current_tool_call_id.as_ref(),
        ctx.current_tool_name.as_ref(),
        message,
    );
}

/// Shared send path for direct emits and [`OutputProgressFn`] callbacks.
fn try_send_tool_progress(
    tx: Option<&tokio::sync::mpsc::UnboundedSender<ToolProgressUpdate>>,
    tool_call_id: Option<&String>,
    tool_name: Option<&String>,
    message: &str,
) {
    let message = message.trim();
    if message.is_empty() {
        return;
    }
    let Some(tx) = tx else {
        return;
    };
    let Some(tool_call_id) = tool_call_id else {
        return;
    };
    let Some(tool_name) = tool_name else {
        return;
    };
    let _ = tx.send(ToolProgressUpdate {
        tool_call_id: tool_call_id.clone(),
        tool_name: tool_name.clone(),
        message: message.to_string(),
    });
}

/// Route one sanitized line to a progress callback (stdout vs stderr).
pub fn dispatch_progress_line(on_line: &OutputProgressFn, line: &str, stderr: bool) {
    if stderr {
        on_line(&format_stderr_progress_line(line));
    } else {
        on_line(line);
    }
}

/// Merge stdout/stderr into sanitized progress lines (stderr tagged).
pub fn collect_output_progress_lines(stdout: &str, stderr: &str) -> Vec<String> {
    let mut lines: Vec<String> = stdout
        .lines()
        .map(sanitize_output_line)
        .filter(|line| !line.is_empty())
        .collect();
    for line in stderr
        .lines()
        .map(sanitize_output_line)
        .filter(|l| !l.is_empty())
    {
        lines.push(format_stderr_progress_line(&line));
    }
    lines
}

/// After a batch (non-streaming) backend completes, push tail lines to the progress hook.
///
/// Callers should still `flush()` the [`ToolProgressTail`] reporter after `execute()` returns
/// (see `terminal.rs`). Only the last [`OUTPUT_TAIL_LINE_COUNT`] lines are pushed to avoid
/// flooding the throttled reporter with full batch output.
pub fn emit_batch_output_progress(options: &ExecuteOptions, stdout: &str, stderr: &str) {
    let Some(on_line) = options.on_output_line.as_ref() else {
        return;
    };
    let lines = collect_output_progress_lines(stdout, stderr);
    if lines.is_empty() {
        return;
    }
    let start = lines.len().saturating_sub(OUTPUT_TAIL_LINE_COUNT);
    for line in &lines[start..] {
        on_line(line);
    }
}

/// Incrementally split byte chunks into lines (lossy UTF-8) for streaming progress.
#[derive(Default)]
pub struct LineSplitter {
    partial: String,
}

impl LineSplitter {
    pub fn push_chunk(&mut self, chunk: &str, mut on_line: impl FnMut(&str)) {
        if chunk.is_empty() {
            return;
        }
        self.partial.push_str(chunk);
        while let Some(newline) = self.partial.find('\n') {
            let line = sanitize_output_line(&self.partial[..newline]);
            self.partial = self.partial[newline + 1..].to_string();
            if !line.is_empty() {
                on_line(&line);
            }
        }
    }

    pub fn finish(&mut self, mut on_line: impl FnMut(&str)) {
        let tail = sanitize_output_line(&self.partial);
        if !tail.is_empty() {
            on_line(&tail);
        }
        self.partial.clear();
    }
}

/// Push raw bytes through the reporter's chunk splitter (preserves partial lines).
pub fn push_bytes_to_progress(reporter: &ToolProgressTail, bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    reporter.note_chunk(&String::from_utf8_lossy(bytes));
}

/// Bridges streaming exec output (Docker, local capture) to `ExecuteOptions`.
pub struct OutputProgressSink {
    on_line: Option<OutputProgressFn>,
    stdout: LineSplitter,
    stderr: LineSplitter,
}

impl OutputProgressSink {
    pub fn from_execute_options(options: &ExecuteOptions) -> Self {
        Self {
            on_line: options.on_output_line.clone(),
            stdout: LineSplitter::default(),
            stderr: LineSplitter::default(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.on_line.is_some()
    }

    pub fn push_stdout(&mut self, bytes: &[u8]) {
        self.push_stream(bytes, false);
    }

    pub fn push_stderr(&mut self, bytes: &[u8]) {
        self.push_stream(bytes, true);
    }

    pub fn finish(&mut self) {
        let Some(on_line) = self.on_line.as_ref() else {
            return;
        };
        let stdout_cb = Arc::clone(on_line);
        self.stdout.finish(|line| stdout_cb(line));
        if let Some(on_line) = self.on_line.as_ref() {
            let stderr_cb = Arc::clone(on_line);
            self.stderr
                .finish(|line| dispatch_progress_line(&stderr_cb, line, true));
        }
    }

    fn push_stream(&mut self, bytes: &[u8], stderr: bool) {
        let Some(on_line) = self.on_line.as_ref() else {
            return;
        };
        let chunk = String::from_utf8_lossy(bytes);
        let on_line = Arc::clone(on_line);
        let splitter = if stderr {
            &mut self.stderr
        } else {
            &mut self.stdout
        };
        splitter.push_chunk(&chunk, |line| {
            dispatch_progress_line(&on_line, line, stderr)
        });
    }
}

/// Stream raw bytes into a [`ToolProgressTail`] (stdout or stderr), preserving partial lines.
pub struct TailByteWriter {
    splitter: LineSplitter,
    reporter: Arc<ToolProgressTail>,
    stderr: bool,
}

impl TailByteWriter {
    pub fn stdout(reporter: Arc<ToolProgressTail>) -> Self {
        Self {
            splitter: LineSplitter::default(),
            reporter,
            stderr: false,
        }
    }

    pub fn stderr(reporter: Arc<ToolProgressTail>) -> Self {
        Self {
            splitter: LineSplitter::default(),
            reporter,
            stderr: true,
        }
    }

    pub fn push(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let chunk = String::from_utf8_lossy(bytes);
        let reporter = Arc::clone(&self.reporter);
        let stderr = self.stderr;
        self.splitter.push_chunk(&chunk, move |line| {
            if stderr {
                reporter.note_line(&format_stderr_progress_line(line));
            } else {
                reporter.note_line(line);
            }
        });
    }

    pub fn finish(&mut self) {
        let reporter = Arc::clone(&self.reporter);
        let stderr = self.stderr;
        self.splitter.finish(move |line| {
            if stderr {
                reporter.note_line(&format_stderr_progress_line(line));
            } else {
                reporter.note_line(line);
            }
        });
    }
}

struct ReporterState {
    lines: Vec<String>,
    last_emit: Instant,
    chunk_splitter: LineSplitter,
}

/// Shared throttled reporter wired from `ToolContext` into shell backends.
pub struct ToolProgressTail {
    tx: tokio::sync::mpsc::UnboundedSender<ToolProgressUpdate>,
    tool_call_id: String,
    tool_name: String,
    state: Mutex<ReporterState>,
}

impl ToolProgressTail {
    /// Build a reporter when the invocation has a progress channel and tool ids.
    pub fn from_context(ctx: &ToolContext) -> Option<Arc<Self>> {
        let tx = ctx.tool_progress_tx.clone()?;
        let tool_call_id = ctx.current_tool_call_id.clone()?;
        let tool_name = ctx.current_tool_name.clone()?;
        Some(Arc::new(Self {
            tx,
            tool_call_id,
            tool_name,
            state: Mutex::new(ReporterState {
                lines: Vec::new(),
                last_emit: Instant::now() - PROGRESS_EMIT_INTERVAL,
                chunk_splitter: LineSplitter::default(),
            }),
        }))
    }

    fn drain_chunk_lines(state: &mut ReporterState, chunk: &str) -> Vec<String> {
        let mut pending = Vec::new();
        state
            .chunk_splitter
            .push_chunk(chunk, |line| pending.push(line.to_string()));
        pending
    }

    fn drain_chunk_finish(state: &mut ReporterState) -> Vec<String> {
        let mut pending = Vec::new();
        state
            .chunk_splitter
            .finish(|line| pending.push(line.to_string()));
        pending
    }

    /// Reporter + backend options — single entry point for tool handlers.
    pub fn reporter_and_options(ctx: &ToolContext) -> (Option<Arc<Self>>, ExecuteOptions) {
        let reporter = Self::from_context(ctx);
        let options = ExecuteOptions {
            on_output_line: reporter
                .as_ref()
                .map(|reporter| Self::line_callback(reporter)),
        };
        (reporter, options)
    }

    /// Callback suitable for `ExecuteOptions::on_output_line`.
    pub fn line_callback(reporter: &Arc<Self>) -> OutputProgressFn {
        let reporter = Arc::clone(reporter);
        Arc::new(move |line| reporter.note_line(line))
    }

    /// Shared progress callback from [`ToolContext`] (web tools, batch backends).
    pub fn progress_fn_from_context(ctx: &ToolContext) -> Option<OutputProgressFn> {
        let tx = ctx.tool_progress_tx.clone()?;
        let tool_call_id = ctx.current_tool_call_id.clone()?;
        let tool_name = ctx.current_tool_name.clone()?;
        Some(Arc::new(move |message: &str| {
            try_send_tool_progress(Some(&tx), Some(&tool_call_id), Some(&tool_name), message);
        }))
    }

    pub(crate) fn emit_progress_fn(on_progress: &Option<OutputProgressFn>, message: &str) {
        if let Some(progress) = on_progress {
            progress(message);
        }
    }

    /// Record one output line and maybe emit a throttled tail update.
    pub fn note_line(&self, raw_line: &str) {
        let line = sanitize_output_line(raw_line);
        if line.is_empty() {
            return;
        }

        let now = Instant::now();
        let should_emit = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.lines.push(line);
            if state.lines.len() > 256 {
                let drain = state.lines.len().saturating_sub(128);
                state.lines.drain(0..drain);
            }
            now.duration_since(state.last_emit) >= PROGRESS_EMIT_INTERVAL
        };

        if should_emit {
            self.emit_tail(now);
        }
    }

    /// Record a raw chunk (PTY / byte-oriented readers), splitting on newlines.
    pub fn note_chunk(&self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        let pending = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            Self::drain_chunk_lines(&mut state, chunk)
        };
        for line in pending {
            self.note_line(&line);
        }
    }

    /// Flush any partial line held by the chunk splitter (call when stream ends).
    pub fn finish_chunk(&self) {
        let pending = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            Self::drain_chunk_finish(&mut state)
        };
        for line in pending {
            self.note_line(&line);
        }
    }

    /// Force a final tail emission (call when command completes).
    pub fn flush(&self) {
        self.finish_chunk();
        self.emit_tail(Instant::now());
    }

    fn emit_tail(&self, now: Instant) {
        let message = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.last_emit = now;
            format_tail_from_lines(&state.lines)
        };

        if message.is_empty() {
            return;
        }

        let _ = self.tx.send(ToolProgressUpdate {
            tool_call_id: self.tool_call_id.clone(),
            tool_name: self.tool_name.clone(),
            message,
        });
    }
}

/// Human-readable exit status for background process monitors (TUI / gateway).
pub fn format_process_exit_status(exit_code: Option<i32>) -> String {
    match exit_code {
        Some(0) => "finished".to_string(),
        Some(code) if code < 0 => "killed".to_string(),
        Some(code) => format!("exited {code}"),
        None => "stopped".to_string(),
    }
}

/// Maximum characters for gateway status messages (Telegram/Slack friendly).
pub const GATEWAY_STATUS_MAX_CHARS: usize = 280;

/// Compact tool-start line for gateway status delivery.
pub fn format_gateway_tool_exec(name: &str) -> String {
    format!("🔧 {name}…")
}

/// Compact tool progress for gateway — last non-empty line, truncated.
pub fn format_gateway_tool_progress(name: &str, message: &str) -> String {
    let detail = message
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or_else(|| message.trim());
    let detail = truncate_command_preview(detail, 140);
    truncate_command_preview(&format!("🔧 {name}: {detail}"), GATEWAY_STATUS_MAX_CHARS)
}

/// Milestone before a remote batch backend runs (SSH, Modal, etc.).
pub fn format_remote_execution_start(backend: &str, command: &str) -> String {
    format!(
        "running on {backend} · {}",
        truncate_command_preview(command, 56)
    )
}

/// Emit a start milestone on batch backends that only stream tail on completion.
pub fn emit_execution_start(options: &ExecuteOptions, backend: &str, command: &str) {
    let Some(on_line) = options.on_output_line.as_ref() else {
        return;
    };
    on_line(&format_remote_execution_start(backend, command));
}

/// Truncate a command string for compact monitor lines (Unicode-safe).
pub fn truncate_command_preview(command: &str, max_chars: usize) -> String {
    let trimmed = command.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let end = trimmed
        .char_indices()
        .nth(max_chars.saturating_sub(1))
        .map(|(idx, _)| idx)
        .unwrap_or(trimmed.len());
    format!("{}…", &trimmed[..end])
}

/// Keep the tail of a progress snippet for gateway/TUI budgets (Unicode-safe).
pub fn truncate_progress_tail(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let start = trimmed
        .char_indices()
        .nth(trimmed.chars().count().saturating_sub(max_chars))
        .map(|(i, _)| i)
        .unwrap_or(0);
    format!("…{}", &trimmed[start..])
}

/// In-place background process monitor line (`📟 id · command` + optional tail).
pub fn format_background_process_monitor(
    process_id: &str,
    command_preview: &str,
    tail: &str,
) -> String {
    let preview = truncate_command_preview(command_preview, 60);
    if tail.trim().is_empty() {
        format!("📟 {process_id} · {preview}")
    } else {
        format!("📟 {process_id} · {preview}\n{tail}")
    }
}

/// Gateway monitor with a configurable tail char budget (Hermes ~500).
pub fn format_background_process_monitor_budget(
    process_id: &str,
    command_preview: &str,
    tail: &str,
    tail_max_chars: usize,
) -> String {
    let tail = truncate_progress_tail(tail, tail_max_chars);
    format_background_process_monitor(process_id, command_preview, &tail)
}

/// Final monitor line after process exit (preserves headline when present).
pub fn format_background_process_finished(
    headline: &str,
    process_id: &str,
    exit_code: Option<i32>,
) -> String {
    let status = format_process_exit_status(exit_code);
    let headline = headline.trim();
    if headline.is_empty() {
        format!("📟 {process_id} ✓ {status}")
    } else {
        format!("{headline} ✓ {status}")
    }
}

/// Returns true when enough time has passed since the last throttled emission.
pub fn should_emit_progress(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|prev| now.duration_since(prev) >= PROGRESS_EMIT_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn format_wait_heartbeat_shows_tail() {
        let msg = format_wait_heartbeat("p1", "line1\nline2\nline3\nline4");
        assert!(msg.contains("p1"));
        assert!(msg.contains("line2"));
        assert!(msg.contains("line4"));
        assert!(!msg.contains("line1"));
    }

    #[test]
    fn line_splitter_handles_partial_lines() {
        let mut splitter = LineSplitter::default();
        let mut lines = Vec::new();
        splitter.push_chunk("hel", |l| lines.push(l.to_string()));
        assert!(lines.is_empty());
        splitter.push_chunk("lo\nwor", |l| lines.push(l.to_string()));
        assert_eq!(lines, vec!["hello".to_string()]);
        splitter.finish(|l| lines.push(l.to_string()));
        assert_eq!(lines, vec!["hello".to_string(), "wor".to_string()]);
    }

    #[test]
    fn sanitize_strips_ansi() {
        assert_eq!(sanitize_output_line("\x1b[31merror\x1b[0m"), "error");
    }

    #[test]
    fn format_process_exit_status_variants() {
        assert_eq!(format_process_exit_status(Some(0)), "finished");
        assert_eq!(format_process_exit_status(Some(42)), "exited 42");
        assert_eq!(format_process_exit_status(Some(-1)), "killed");
        assert_eq!(format_process_exit_status(None), "stopped");
    }

    #[test]
    fn truncate_progress_tail_keeps_suffix() {
        let body = "a".repeat(600);
        let out = truncate_progress_tail(&body, 500);
        assert!(out.starts_with('…'));
        assert!(out.chars().count() <= 501);
    }

    #[test]
    fn gateway_bg_monitor_respects_budget() {
        let tail = "x".repeat(800);
        let msg = format_background_process_monitor_budget("p1", "npm run dev", &tail, 500);
        assert!(msg.contains("p1"));
        assert!(msg.chars().count() < 700);
    }

    #[test]
    fn emit_batch_output_progress_sends_tail_only() {
        let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&lines);
        let options = ExecuteOptions {
            on_output_line: Some(Arc::new(move |line| {
                captured.lock().unwrap().push(line.to_string());
            })),
        };
        emit_batch_output_progress(&options, "line1\nline2\nline3\nline4", "err1\n");
        let got = lines.lock().unwrap().clone();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0], "line3");
        assert_eq!(got[1], "line4");
        assert_eq!(got[2], "[stderr] err1");
    }

    #[test]
    fn push_bytes_to_progress_preserves_partial_lines() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let reporter = Arc::new(ToolProgressTail {
            tx,
            tool_call_id: "tc1".into(),
            tool_name: "terminal".into(),
            state: Mutex::new(ReporterState {
                lines: Vec::new(),
                last_emit: Instant::now() - PROGRESS_EMIT_INTERVAL,
                chunk_splitter: LineSplitter::default(),
            }),
        });
        push_bytes_to_progress(&reporter, b"hel");
        assert!(rx.try_recv().is_err());
        push_bytes_to_progress(&reporter, b"lo\nok");
        let event = rx.try_recv().expect("line emitted");
        assert!(event.message.contains("hello"));
    }

    #[test]
    fn format_browser_milestone_includes_action_and_detail() {
        let msg = format_browser_milestone("navigating", "https://example.com/page");
        assert!(msg.contains("browser:"));
        assert!(msg.contains("navigating"));
        assert!(msg.contains("example.com"));
    }

    #[test]
    fn format_activity_notices_are_non_empty() {
        assert!(format_compression_started().contains("Compressing"));
        assert!(format_compression_done(42).contains("42"));
        assert!(format_compression_circuit_breaker(3).contains("structural"));
        assert!(format_approval_waiting("rm -rf /tmp/foo").contains("approval"));
    }

    #[test]
    fn format_gateway_tool_progress_uses_last_line() {
        let msg =
            format_gateway_tool_progress("terminal", "line1\nline2\nCompiling edgecrab v0.9.0");
        assert!(msg.contains("terminal"));
        assert!(msg.contains("Compiling"));
        assert!(!msg.contains("line1"));
    }

    #[test]
    fn format_remote_execution_start_truncates_command() {
        let msg = format_remote_execution_start("SSH", &"x".repeat(120));
        assert!(msg.contains("SSH"));
        assert!(msg.contains('…'));
    }

    #[test]
    fn emit_execution_start_skips_without_hook() {
        emit_execution_start(&ExecuteOptions::default(), "Modal", "cargo build");
    }

    #[test]
    fn format_browser_milestone_omits_empty_detail() {
        let msg = format_browser_milestone("connecting", "   ");
        assert_eq!(msg, "browser: connecting…");
    }

    #[test]
    fn emit_tool_progress_skips_empty_and_delivers_when_wired() {
        use crate::registry::ToolContext;

        let mut ctx = ToolContext::test_context();
        emit_tool_progress(&ctx, "   ");
        emit_tool_progress(&ctx, "should not send");

        let (tx, mut rx) = mpsc::unbounded_channel();
        ctx.tool_progress_tx = Some(tx);
        ctx.current_tool_call_id = Some("tc-progress".into());
        ctx.current_tool_name = Some("browser_click".into());

        emit_tool_progress(&ctx, "browser: click · @e1");
        let update = rx.try_recv().expect("progress delivered");
        assert_eq!(update.tool_call_id, "tc-progress");
        assert_eq!(update.tool_name, "browser_click");
        assert!(update.message.contains("@e1"));

        let progress_fn = ToolProgressTail::progress_fn_from_context(&ctx).expect("fn");
        progress_fn("browser: hover · @e2");
        let update2 = rx.try_recv().expect("callback path works");
        assert!(update2.message.contains("@e2"));
    }

    #[test]
    fn format_minimal_tool_indicator_includes_tail_and_elapsed() {
        let text = format_minimal_tool_indicator("terminal", "cargo build", Some(12), "Compiling…");
        assert!(text.contains("cargo build"));
        assert!(text.contains("(12s)"));
        assert!(text.contains("Compiling"));
    }

    #[test]
    fn format_search_milestone_truncates_long_query() {
        let msg = format_search_milestone(&"x".repeat(200));
        assert!(msg.contains('…'));
        assert!(msg.starts_with("searching"));
    }

    #[test]
    fn note_chunk_preserves_partial_lines_across_calls() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let reporter = Arc::new(ToolProgressTail {
            tx,
            tool_call_id: "tc1".into(),
            tool_name: "terminal".into(),
            state: Mutex::new(ReporterState {
                lines: Vec::new(),
                last_emit: Instant::now() - PROGRESS_EMIT_INTERVAL,
                chunk_splitter: LineSplitter::default(),
            }),
        });

        reporter.note_chunk("hel");
        assert!(rx.try_recv().is_err());
        reporter.note_chunk("lo\nwor");
        let event = rx.try_recv().expect("completed line emitted");
        assert!(event.message.contains("hello"));
    }

    #[tokio::test]
    async fn throttles_progress_emissions() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let reporter = Arc::new(ToolProgressTail {
            tx,
            tool_call_id: "tc1".into(),
            tool_name: "terminal".into(),
            state: Mutex::new(ReporterState {
                lines: Vec::new(),
                last_emit: Instant::now() - PROGRESS_EMIT_INTERVAL,
                chunk_splitter: LineSplitter::default(),
            }),
        });

        reporter.note_line("first");
        reporter.note_line("second");
        let first = rx.try_recv().expect("first emit");
        assert!(first.message.contains("first"));

        reporter.note_line("third");
        assert!(rx.try_recv().is_err(), "should throttle second emit");

        reporter.flush();
        let flushed = rx.try_recv().expect("flush emit");
        assert!(flushed.message.contains("third"));
    }
}
