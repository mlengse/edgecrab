# 002 — Terminal & Process Tools (Observability)

Deep audit of shell execution visibility. **Code is law:** the terminal tool is designed for **batch return to the LLM**, not live human mirroring.

## Foreground `terminal` tool

**Entry:** [terminal.rs](../../crates/edgecrab-tools/src/tools/terminal.rs) `TerminalTool::execute` L197+

### Args that affect visibility

| Field | Default | Effect on UX |
|-------|---------|--------------|
| `timeout_seconds` / `timeout` | 120s | Hard cap 600s — user sees spinner entire duration |
| `background` | false | If true → immediate JSON `{process_id}`; output elsewhere |
| `pty` | false | PTY poll loop; still batch return |
| `workdir` | ctx.cwd | Shown in result header only |

### Execution flow (foreground)

```text
detect_file_io_antipattern (soft warning appended to result)
  → scan_command (security)
  → request_command_approval (may block TUI in WaitingForApproval)
  → backend.execute() OR local_pty.execute()
  → strip_ansi_escapes + redact_output
  → terminal_result_header + body
  → return String to conversation loop
```

**No call to `ctx.emit_progress()` anywhere in this path.**

### Machine-readable header

The LLM and `format_terminal_result` depend on this prefix:

```73:83:crates/edgecrab-tools/src/tools/terminal.rs
fn terminal_result_header(...) -> String {
    format!("[terminal_result status={status} backend={} cwd={} exit_code={exit_code}]", ...)
}
```

TUI compacts to `✓ 0  first-output-line` via [tool_display.rs L1823–1858](../../crates/edgecrab-cli/src/tool_display.rs).

### What the user sees vs what the LLM gets

| Phase | User (TUI) | LLM (`Message::tool`) |
|-------|------------|------------------------|
| Running | Status bar: `💻 terminal $ cmd ···`; optional placeholder line | Nothing |
| Done | One-line preview + duration in placeholder upgrade | Full stdout/stderr (possibly truncated) |
| Verbose mode | Extra lines via `build_tool_verbose_lines_width` | Same |

Truncation: `max_terminal_output` config; truncation noted in header ([terminal.rs L362–390](../../crates/edgecrab-tools/src/tools/terminal.rs)).

## Background processes

### Registration & drain

[start_background_process](../../crates/edgecrab-tools/src/tools/process.rs) L120–205:

1. Optional approval gate
2. `ProcessTable::register`
3. Spawn local or remote child
4. `drain_reader` tasks append to ring buffer

### ProcessTable ring buffer

[process_table.rs](../../crates/edgecrab-tools/src/process_table.rs):

| Constant / behavior | Value | UX impact |
|---------------------|-------|-----------|
| `RING_CAPACITY` | 500 lines | Older output dropped silently |
| `append_output_chunk` | Handles `\r` overwrite (progress bars) | Works for agent polling, invisible to TUI |
| GC task | 5min interval, 30min TTL | Process records expire |

PTY carriage-return handling (progress bars in background):

```546:565:crates/edgecrab-tools/src/process_table.rs
'\r' => {
    if matches!(chars.peek(), Some('\n')) { ... }
    else { rec.carriage_return_pending = true; }
}
```

### Agent polling tools (not user polling)

| Tool | Blocks agent? | Stream events during run? |
|------|---------------|---------------------------|
| `get_process_output` | No (instant read) | None |
| `wait_for_process` | **Yes** — up to 3600s | None — single long `ToolExec` spinner |
| `list_processes` | No | None |
| `kill_process` | Brief | None |

`wait_for_process` loop ([process.rs L947–988](../../crates/edgecrab-tools/src/tools/process.rs)): 500ms sleep, respects `ctx.cancel`, returns snapshot at end or timeout message.

### Watch patterns — implemented, unwired

`run_process` accepts `watch_patterns` and passes `ctx.watch_notification_tx` ([process.rs L189](../../crates/edgecrab-tools/src/tools/process.rs)).

But production context always sets:

```390:390:crates/edgecrab-core/src/conversation.rs
        watch_notification_tx: None,
```

So `WatchEvent` notifications from background drains **never reach TUI or gateway**.

## Execution backends detail

### Local persistent shell

[backends/local.rs L438–477](../../crates/edgecrab-tools/src/tools/backends/local.rs):

- Reads until fence sentinel
- Checks cancel + timeout on each 500ms read slice
- Exit codes: 130 (cancel), 124 (timeout)
- macOS prompt stall timeout (separate path)

**Output accumulation is in-memory until command completes.**

### Local PTY

[local_pty.rs L108–148](../../crates/edgecrab-tools/src/local_pty.rs):

- 25ms poll loop
- Same batch return semantics

Tool schema explicitly warns: *"Full-screen terminal UIs remain unsupported because the agent does not observe screen state."* ([terminal.rs L156–157](../../crates/edgecrab-tools/src/tools/terminal.rs))

### Remote backends

Background remote processes poll log file every **2 seconds** ([process.rs L497–514](../../crates/edgecrab-tools/src/tools/process.rs)). Agent-visible snapshots lag; user sees nothing unless agent polls.

## Approval gate visibility

Dangerous commands block at [approval_runtime.rs](../../crates/edgecrab-tools/src/approval_runtime.rs) before any execution starts.

TUI: `DisplayState::WaitingForApproval` — user sees approval overlay, not partial execution. This is correct safety UX but adds perceived “stuck” time with no progress indicator beyond overlay state.

## ToolProgress API (unused for shell)

```424:443:crates/edgecrab-tools/src/registry.rs
pub fn emit_progress(&self, message: impl Into<String>) {
    let Some(tx) = &self.tool_progress_tx else { return; };
    // ... sends ToolProgressUpdate → StreamEvent::ToolProgress
}
```

**Grep result:** zero calls from `terminal.rs`, `process.rs`, `web.rs`, `browser.rs`, etc. Only [mixture_of_agents.rs L367](../../crates/edgecrab-tools/src/tools/mixture_of_agents.rs) sends progress directly.

## Honest capability matrix

| User expectation | Actual behavior | Gap severity |
|------------------|-----------------|--------------|
| See compiler/test output live | Output buffered until tool returns | **Critical** |
| See background server logs | Only if agent calls `get_process_output` next turn | **High** |
| Know watch pattern matched | Infrastructure exists, channel unwired | **High** |
| Expand full terminal output inline | Verbose mode adds lines; no progressive disclosure for terminal | **Medium** |
| See which parallel shell is active | Status bar summarizes; not a list | **Medium** |
| Cancel long command | Works (^C / interrupt); exit 130 | **OK** |
| See approval wait | Overlay yes; no countdown | **Low** |

## Cross-references

- Data flow → [001-data-flow-map.md](001-data-flow-map.md)
- TUI rendering of tool lines → [003-tui-visibility-layer.md](003-tui-visibility-layer.md)
- Fix priorities → [007-implementation-roadmap.md](007-implementation-roadmap.md)
- Code index → [008-cross-ref-index.md](008-cross-ref-index.md)
