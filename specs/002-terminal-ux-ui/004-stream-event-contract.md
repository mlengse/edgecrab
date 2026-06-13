# 004 — StreamEvent Contract (Producers × Consumers)

`StreamEvent` lives in [agent.rs](../../crates/edgecrab-core/src/agent.rs) — re-exported as `edgecrab_core::StreamEvent`.

Maps every variant: who emits it, who consumes it, and wiring status after `feat/terminal-ux-live-progress`.

## Consumer matrix

| Consumer | Entry point | Notes |
|----------|-------------|-------|
| CLI TUI | [app.rs `forward_stream_event_to_tui`](../../crates/edgecrab-cli/src/app.rs) | Full match; in-place tool progress |
| Gateway | [event_processor.rs](../../crates/edgecrab-gateway/src/event_processor.rs) | Throttled status + stream consumer |
| SDK (Python/Node) | Subset + new variants | tool_progress, activity_notice, bg process, steer |

## Variant catalog

### LLM output

| Variant | Emitted? | TUI | Gateway | Notes |
|---------|----------|-----|---------|-------|
| `Token(String)` | ✅ | ✅ | ✅ Stream consumer | Non-streaming path: batch only |
| `Reasoning(String)` | ✅ | ✅ | ✅ If `show_reasoning` | Hidden unless `/reasoning show` |

### Tool lifecycle

| Variant | Emitted? | TUI | Gateway | Notes |
|---------|----------|-----|---------|-------|
| `ToolExec { tool_call_id, name, args_json }` | ✅ | ✅ Placeholder + status | ✅ Status | Parallel: one per call |
| `ToolProgress { tool_call_id, name, message }` | ✅ | ✅ In-place update | ✅ Throttled status | terminal, execute_code, web, browser, remote start/tail |
| `ToolDone { … }` | ✅ | ✅ Upgrade placeholder | ⚠️ Errors + notable completions | Preview ≠ full result |

Central progress path:

```text
ToolContext.emit_progress(msg)
  → try_send_tool_progress (tool_progress_tail.rs)
  → ToolProgressUpdate on tool_progress_tx
  → make_tool_progress_tx bridge (conversation.rs)
  → StreamEvent::ToolProgress
  → TUI check_responses / gateway event_processor
```

Parallel dispatch clones `tool_progress_tx` into spawned tasks ([conversation.rs](../../crates/edgecrab-core/src/conversation.rs)).

### Background processes

| Variant | Emitted? | Producer | TUI | Gateway |
|---------|----------|----------|-----|---------|
| `BackgroundProcessTail` | ✅ | `forward_process_watch_event` | ✅ `bg_process_lines` | ✅ Throttled |
| `BackgroundProcessFinished` | ✅ | Same | ✅ In-place finish | ✅ Status |

`watch_notification_tx` is wired per turn when `event_tx` is present.

### Activity notices

| Variant | Emitted? | Examples |
|---------|----------|----------|
| `ActivityNotice(String)` | ✅ | Compression start/done, circuit breaker, approval waiting, watch pattern match |

Formatters live in [`tool_progress_tail.rs`](../../crates/edgecrab-tools/src/tool_progress_tail.rs) and [`process_table.rs`](../../crates/edgecrab-tools/src/process_table.rs) (`format_watch_activity_notice`).

### Turn completion

| Variant | Semantics |
|---------|-----------|
| `RunFinished { outcome }` | Harness terminal state |
| `Footer(String)` | Per-turn mutation log |
| `Done` | Transport complete (not semantic) |
| `Error(String)` | Turn failed |

Handle **`RunFinished` before `Done`**.

### Interactive overlays

| Variant | TUI | Gateway |
|---------|-----|---------|
| `Clarify { … }` | Modal | Pending interaction message |
| `Approval { … }` | Overlay + `ActivityNotice` | Pending interaction message |
| `SecretRequest { … }` | Handler exists | Env fallback |

### System signals

| Variant | Status |
|---------|--------|
| `ContextPressure` | ✅ Once/turn at 85% |
| `SteerPending { count }` | ✅ From `Agent::send_steering()` |
| `SteerApplied { message }` | ✅ System notice |
| `HookEvent` | Registry only — not transcript |

### Compression visibility (implemented)

| Signal | StreamEvent |
|--------|-------------|
| Compression started | `ActivityNotice` (`format_compression_started`) |
| Compression complete | `ActivityNotice` (`format_compression_done`) |
| Circuit breaker (3 LLM failures) | `ActivityNotice` (`format_compression_circuit_breaker`) |
| Approaching threshold (85%) | `ContextPressure` |

## Remaining partial variants

| Variant | Status |
|---------|--------|
| `SecretRequest` | TUI handler; no core producer in CLI path |
| `ToolOutput` (proposed) | Deferred — `ToolProgress` sufficient |

## Cross-references

- Data flow → [001-data-flow-map.md](001-data-flow-map.md)
- Assessment → [005-honest-assessment.md](005-honest-assessment.md)
- Roadmap → [007-implementation-roadmap.md](007-implementation-roadmap.md)
