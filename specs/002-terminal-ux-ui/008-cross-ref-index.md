# 008 — Cross-Reference Index (Code Is Law)

Master index of symbols, files, and spec documents. Use for navigation during implementation.

## Spec documents

| ID | Path | Topic |
|----|------|-------|
| 000 | [000-overview.md](000-overview.md) | Executive summary |
| 001 | [001-data-flow-map.md](001-data-flow-map.md) | End-to-end flow |
| 002 | [002-terminal-and-process-tools.md](002-terminal-and-process-tools.md) | Shell execution |
| 003 | [003-tui-visibility-layer.md](003-tui-visibility-layer.md) | TUI rendering |
| 004 | [004-stream-event-contract.md](004-stream-event-contract.md) | StreamEvent catalog |
| 005 | [005-honest-assessment.md](005-honest-assessment.md) | Gap analysis |
| 006 | [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md) | Debug scenarios |
| 007 | [007-implementation-roadmap.md](007-implementation-roadmap.md) | Prioritized fixes |

## Core types & enums

| Symbol | File | Lines |
|--------|------|-------|
| `StreamEvent` | [agent.rs](../../crates/edgecrab-core/src/agent.rs) | L1956–2117 |
| `chat_streaming` | [agent.rs](../../crates/edgecrab-core/src/agent.rs) | L775–849 |
| `Agent::interrupt` | [agent.rs](../../crates/edgecrab-core/src/agent.rs) | L927–933 |
| `execute_loop` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L590–2494 |
| `process_response` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L4252–4641 |
| `dispatch_single_tool` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L4788+ |
| `emit_tool_done` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L2865–2883 |
| `make_tool_progress_tx` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L2886–2901 |
| `build_tool_context` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L321–393 |
| `api_call_streaming` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L3286–3451 |
| `should_use_native_streaming` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L295–310 |
| `ContextPressure` emit | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L1572–1587 |
| Parallel `event_tx: None` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L4371 |
| `ToolProgressMode` | [config.rs](../../crates/edgecrab-core/src/config.rs) | L1810+ |
| `CompressionStatus` | [compression.rs](../../crates/edgecrab-core/src/compression.rs) | L218–307 |
| `drain_pending_steers` | [steering.rs](../../crates/edgecrab-core/src/steering.rs) | L125–151 |
| `Message`, `Role` | [message.rs](../../crates/edgecrab-types/src/message.rs) | L10–248 |
| `RunOutcome` | [harness.rs](../../crates/edgecrab-types/src/harness.rs) | L141–175 |

## Tools layer

| Symbol | File | Lines |
|--------|------|-------|
| `TerminalTool::execute` | [terminal.rs](../../crates/edgecrab-tools/src/tools/terminal.rs) | L197+ |
| `terminal_result_header` | [terminal.rs](../../crates/edgecrab-tools/src/tools/terminal.rs) | L73–83 |
| `start_background_process` | [process.rs](../../crates/edgecrab-tools/src/tools/process.rs) | L120–205 |
| `wait_for_process` loop | [process.rs](../../crates/edgecrab-tools/src/tools/process.rs) | L947–988 |
| `ProcessTable::append_output` | [process_table.rs](../../crates/edgecrab-tools/src/process_table.rs) | L523–575 |
| `RING_CAPACITY` | [process_table.rs](../../crates/edgecrab-tools/src/process_table.rs) | ~500 lines |
| `ToolContext::emit_progress` | [registry.rs](../../crates/edgecrab-tools/src/registry.rs) | L424–443 |
| `watch_notification_tx` field | [registry.rs](../../crates/edgecrab-tools/src/registry.rs) | L328–329 |
| Local shell execute loop | [backends/local.rs](../../crates/edgecrab-tools/src/tools/backends/local.rs) | L438–477 |
| PTY execute loop | [local_pty.rs](../../crates/edgecrab-tools/src/local_pty.rs) | L108–148 |
| MoA progress (reference impl) | [mixture_of_agents.rs](../../crates/edgecrab-tools/src/tools/mixture_of_agents.rs) | L367 |
| `request_command_approval` | [approval_runtime.rs](../../crates/edgecrab-tools/src/approval_runtime.rs) | L157+ |

## CLI TUI

| Symbol | File | Lines |
|--------|------|-------|
| `forward_stream_event_to_tui` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L510–771 |
| `forward_agent_stream_to_tui` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L773–870 |
| `process_input` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | ~L12155 |
| `check_responses` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L13038+ |
| `ToolExec` handler | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L13177–13257 |
| `ToolProgress` handler | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L13259–13323 |
| `ToolDone` handler | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L13325–13424 |
| `tick_spinner` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L14522–14634 |
| `DisplayState` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L2592–2680 |
| `render_status_bar` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L23975+ |
| `render_output` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L23541+ |
| `event_loop` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L31658+ |
| `should_render_tool_call` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L19603–19611 |
| `flush_buffered_assistant_output` | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L5671–5705 |
| `SteerPending` drop | [app.rs](../../crates/edgecrab-cli/src/app.rs) | L748–752 |
| `/verbose` dispatch | [commands.rs](../../crates/edgecrab-cli/src/commands.rs) | L817–828 |
| `/stream` dispatch | [commands.rs](../../crates/edgecrab-cli/src/commands.rs) | L1110–1113 |
| `format_terminal_result` | [tool_display.rs](../../crates/edgecrab-cli/src/tool_display.rs) | L1823–1858 |
| `build_tool_running_line_width` | [tool_display.rs](../../crates/edgecrab-cli/src/tool_display.rs) | ~L1414 |
| `build_tool_done_line_width` | [tool_display.rs](../../crates/edgecrab-cli/src/tool_display.rs) | L1288+ |

## Gateway

| Symbol | File | Lines |
|--------|------|-------|
| Event processor module docs | [event_processor.rs](../../crates/edgecrab-gateway/src/event_processor.rs) | L1–43 |
| `format_context_pressure_status` | [event_processor.rs](../../crates/edgecrab-gateway/src/event_processor.rs) | L58–70 |
| ToolProgress handler | [event_processor.rs](../../crates/edgecrab-gateway/src/event_processor.rs) | ~L280 |

## edgequake_llm integration

| Concern | File | Lines |
|---------|------|-------|
| `LLMProvider` trait usage | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L62–63 |
| `chat_with_tools_stream` | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L3298–3300 |
| `StreamChunk` handling | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L3339–3390 |
| `ToolCallDelta` silent acc | [conversation.rs](../../crates/edgecrab-core/src/conversation.rs) | L3361–3381 |

## Event flow quick reference

```text
terminal.rs execute()
  → (no emit_progress)
  → dispatch returns String
  → emit_tool_done → StreamEvent::ToolDone
  → forward_stream_event_to_tui
  → check_responses → build_tool_done_line_width
  → format_terminal_result (one line)

MoA (working progress path):
  → ToolProgressUpdate
  → make_tool_progress_tx
  → StreamEvent::ToolProgress
  → check_responses → placeholder in-place update
```

## Related external specs

| Spec | Relevance |
|------|-----------|
| [specs/05-improve-ux-tui.md](../05-improve-ux-tui.md) | Width-adaptive display, approval overlay |
| [specs/improve_plan/04-error-guidance.md](../improve_plan/04-error-guidance.md) | Tool error self-healing |
| [specs/steering/](../steering/) | Mission steering UX |
| [AGENTS.md](../../AGENTS.md) | Architecture overview |

## Cross-links between audit docs

```text
000-overview ──┬── 001-data-flow-map ──┬── 002-terminal-tools
               │                       └── 004-stream-event
               ├── 003-tui-visibility
               ├── 005-assessment ────── 006-stuck-playbook
               └── 007-roadmap ───────── 008-index (this file)
```
