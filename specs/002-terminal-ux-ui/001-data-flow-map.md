# 001 — Data Flow Map (Agent Turn → Terminal → TUI)

Code-is-law map of how visibility events propagate. All paths assume **`chat_streaming()`** — the TUI entry point. Non-streaming `chat()` / `run_conversation()` pass `event_tx: None` and emit **zero** live events.

## High-level sequence

```text
User Enter (app.rs process_input ~L12155)
  │
  ├─ is_processing=true, DisplayState=AwaitingFirstToken
  │
  └─ tokio::spawn agent.chat_streaming(msg, chunk_tx)
       │
       ├─ internal (event_tx, event_rx) channel          agent.rs L794–805
       ├─ forwarder: event_rx → chunk_tx
       └─ execute_loop(..., event_tx)                    conversation.rs L590+
            │
            loop (ReAct):
              ├─ compress? (silent except ContextPressure)  L1448–1587
              ├─ api_call_streaming OR api_call_with_retry
              │    ├─ Token / Reasoning → event_tx         L3340–3356
              │    └─ ToolCallDelta accumulated silently   L3361–3381
              ├─ tool_calls?
              │    ├─ ToolExec (each call)                 L4314–4319
              │    ├─ dispatch_single_tool
              │    │    └─ terminal.rs: batch exec, no progress
              │    ├─ ToolDone + result_preview            emit_tool_done L2865
              │    └─ drain steers → SteerApplied          L2123–2154
              └─ text → LoopAction::Done
            │
            Footer? → RunFinished → (chat_streaming sends Done)

chunk_tx ──► forward_stream_event_to_tui (app.rs L510)
              └─ AgentResponse mpsc ──► check_responses (L13038)
                    └─ render() → output pane + status bar
```

## Layer 1 — Core agent (`edgecrab-core`)

| Step | File | Lines | Observable output |
|------|------|-------|-------------------|
| Streaming entry | [agent.rs](../../crates/edgecrab-core/src/agent.rs) | L775–849 | Internal channel; `Done` at end |
| ReAct loop | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L590–2494 | All `StreamEvent`s |
| Native LLM stream | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L3286–3451 | `Token`, `Reasoning` |
| Tool dispatch | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L4252–4641 | `ToolExec`, `ToolDone` |
| Progress bridge | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L2886–2901 | `ToolProgress` (if tool calls `emit_progress`) |
| Tool context build | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L321–393 | `watch_notification_tx: None` **hardcoded** |

### edgequake_llm boundary

Messages convert OpenAI-shaped `edgecrab_types::Message` → `edgequake_llm::ChatMessage` before API calls ([conversation.rs L2497+](../../crates/edgecrab-core/src/conversation.rs)).

Streaming gate: `should_use_native_streaming` requires `streaming_enabled`, `event_tx`, and provider tool-stream support ([conversation.rs L295–310](../../crates/edgecrab-core/src/conversation.rs)).

**Gap:** `StreamChunk::ToolCallDelta` is assembled into `LLMResponse.tool_calls` but never forwarded to UI until post-parse `ToolExec`.

## Layer 2 — Tools (`edgecrab-tools`)

| Tool | Visibility behavior | Code |
|------|---------------------|------|
| `terminal` (foreground) | Blocks until complete; returns full stdout blob | [terminal.rs L197–402](../../crates/edgecrab-tools/src/tools/terminal.rs) |
| `terminal` (background=true) | Delegates to `start_background_process` | [terminal.rs L210–239](../../crates/edgecrab-tools/src/tools/terminal.rs) |
| `run_process` | Registers in `ProcessTable`, drains stdout async | [process.rs L120–205](../../crates/edgecrab-tools/src/tools/process.rs) |
| `get_process_output` | Agent polls ring buffer (500 lines) | [process.rs](../../crates/edgecrab-tools/src/tools/process.rs) |
| `wait_for_process` | **Blocks tool dispatch** 500ms poll loop | [process.rs L947–988](../../crates/edgecrab-tools/src/tools/process.rs) |
| `emit_progress()` API | Exists on `ToolContext` | [registry.rs L424–443](../../crates/edgecrab-tools/src/registry.rs) |
| Actual `emit_progress` callers | **Only** `mixture_of_agents.rs` | grep confirms |

### Execution backends (all batch-only to UI)

| Backend | Timeout / cancel | Output path |
|---------|------------------|-------------|
| Local persistent shell | 120s default, 600s max; cancel→130 | [backends/local.rs L438–477](../../crates/edgecrab-tools/src/tools/backends/local.rs) |
| Local PTY | 25ms poll loop | [local_pty.rs L108–148](../../crates/edgecrab-tools/src/local_pty.rs) |
| Remote (Docker/SSH/Modal) | 2s log poll for background | [process.rs L497–514](../../crates/edgecrab-tools/src/tools/process.rs) |

## Layer 3 — TUI bridge (`edgecrab-cli`)

| Function | Role | Lines |
|----------|------|-------|
| `forward_stream_event_to_tui` | `StreamEvent` → `AgentResponse` | [app.rs L510–771](../../crates/edgecrab-cli/src/app.rs) |
| `check_responses` | State mutations + output lines | [app.rs L13038+](../../crates/edgecrab-cli/src/app.rs) |
| `tick_spinner` | 80ms spinner; elapsed refresh at 3s | [app.rs L14522–14634](../../crates/edgecrab-cli/src/app.rs) |
| `render_status_bar` | Spinner, model, tokens, steering | [app.rs L23975+](../../crates/edgecrab-cli/src/app.rs) |
| `build_tool_running_line_width` | In-flight placeholder | [tool_display.rs](../../crates/edgecrab-cli/src/tool_display.rs) |
| `format_terminal_result` | Done-line: `✓ 0  first-line` | [tool_display.rs L1823–1858](../../crates/edgecrab-cli/src/tool_display.rs) |

### Dropped / ignored events in bridge

| Event | Bridge behavior |
|-------|-----------------|
| `SteerPending` | Debug log only; no `AgentResponse` ([app.rs L748–752](../../crates/edgecrab-cli/src/app.rs)) |
| `HookEvent` | Routed to `HookRegistry`; not in transcript ([app.rs L717–728](../../crates/edgecrab-cli/src/app.rs)) |
| `ModelTransferComplete` | Handled via background op path ([app.rs L765–767](../../crates/edgecrab-cli/src/app.rs)) |

## Layer 4 — Gateway (brief)

[ event_processor.rs](../../crates/edgecrab-gateway/src/event_processor.rs) maps the same `StreamEvent`s to platform status messages:

- `ToolExec` → `"🔧 {name}…"` when `tool_progress=true`
- `ToolProgress` → status update
- Tokens → `GatewayStreamConsumer` progressive edit

**Same terminal opacity:** gateway never receives live shell stdout either.

## Parallel tool dispatch blind spot

When tools are parallel-safe, spawned tasks use `event_tx: None`:

```4371:4371:crates/edgecrab-core/src/conversation.rs
                        event_tx: None, // ToolExec event already sent before dispatch
```

`ToolExec` is sent **before** spawn ([L4314–4319](../../crates/edgecrab-core/src/conversation.rs)), but **`ToolProgress` cannot fire** for parallel tools because `make_tool_progress_tx` is not wired into parallel `DispatchContext`.

## Two-channel mental model

| Channel | Audience | Content | Live? |
|---------|----------|---------|-------|
| `StreamEvent` → TUI | Human | Tokens, tool lifecycle, notices | Partial |
| `Message::tool(...)` | LLM | Full tool result strings | After completion |

**Root design choice:** terminal stdout is LLM-facing first, human-facing second (one-line preview only).

## Cross-references

- Terminal detail → [002-terminal-and-process-tools.md](002-terminal-and-process-tools.md)
- TUI states → [003-tui-visibility-layer.md](003-tui-visibility-layer.md)
- Event catalog → [004-stream-event-contract.md](004-stream-event-contract.md)
- Stuck scenarios → [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md)
