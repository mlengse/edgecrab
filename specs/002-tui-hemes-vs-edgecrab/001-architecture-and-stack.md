# 001 — Architecture & Stack

## Stack comparison

| | Hermes | EdgeCrab |
|---|--------|----------|
| TUI framework | Custom Ink fork (`ui-tui/packages/hermes-ink`) | ratatui + crossterm |
| Agent coupling | **Out-of-process** JSON-RPC (`tui_gateway/server.py`) | **In-process** `StreamEvent` → `AgentResponse` |
| Progress layer | Python callbacks → gateway events | `tool_progress_tail.rs` → `StreamEvent::ToolProgress` |
| Disclosure config | RPC `details_mode` / `details_mode.*` | YAML `display.shelf_details` + `shelf_details.rs` |
| Skin / theming | Gateway `GatewaySkin` → Ink theme | `skin_engine.rs` + `theme.rs` |

---

## Process model

### Hermes (2+ processes)

**Pros**
- UI crash ≠ agent death (`gatewayRecovery.ts`)
- React component model: hooks, memo, dedicated test files per concern
- Same gateway serves desktop app, web dashboard PTY, Ink TUI

**Cons**
- Startup latency; Node memory footprint
- Documented OOM from verbose tool trails → Ink node explosion (`ui-tui/src/config/limits.ts` L6–17, issue #34095)
- RPC boundary; event batching (`STREAM_BATCH_MS` in `turnController.ts`)

### EdgeCrab (single binary)

**Pros**
- No JSON-RPC hop; `forward_stream_event_to_tui` in `app.rs` is direct
- Rust avoids Node-style render-tree OOM
- Lower ops burden for CLI users

**Cons**
- **`app.rs` monolith: 38,243 lines** (June 2026 baseline) — now **34,225** core + **1,668** in `app/` submodules
- UI changes are high-risk; render paths hard to unit-test in isolation
- No separate UI process to restart without killing the agent

---

## File gravity (code is law)

| Artifact | Lines | Role |
|----------|-------|------|
| `edgecrab-cli/src/app.rs` | **34,225** | TUI core (shrinking) |
| `edgecrab-cli/src/app/response_dispatch.rs` | **1,206** | `check_responses` |
| `edgecrab-cli/src/app/stream_forward.rs` | **462** | StreamEvent → AgentResponse |
| `edgecrab-cli/src/transcript.rs` | **596** | Transcript render |
| `edgecrab-cli/src/activity_shelf.rs` | 768 | Shelf renderer |
| `edgecrab-cli/src/turn_activity.rs` | 776 | Turn-scoped live state |
| `edgecrab-cli/src/details_panel.rs` | ~488 | `/details` picker overlay |
| `edgecrab-cli/src/process_tail_panel.rs` | ~67 | `/tail` overlay |
| `hermes-agent/ui-tui/src/components/thinking.tsx` | 1,224 | Shelf UI |
| `hermes-agent/ui-tui/src/components/agentsOverlay.tsx` | 1,073 | `/agents` dashboard |
| `hermes-agent/ui-tui/src/app/turnController.ts` | 1,009 | Turn state |
| `hermes-agent/tui_gateway/server.py` | 10,185 | RPC + agent bridge |

Hermes UI: **202** TS/TSX source files, **71** files under `src/__tests__/`.

EdgeCrab extracted shelf modules (good) but **~95% of TUI logic remains in `app.rs`**.

---

## Event vocabulary mapping

| Hermes gateway event | EdgeCrab `StreamEvent` | Primary consumer |
|---------------------|------------------------|------------------|
| `tool.generating` | `ToolGenerating` | `turn_activity.rs`, `app.rs` |
| `tool.start` | `ToolExec` | shelf + transcript placeholder |
| `tool.progress` | `ToolProgress` | shelf + in-place line |
| `tool.complete` | `ToolDone` | transcript upgrade |
| `reasoning.delta` | reasoning stream (opt-in) | `/reasoning show` |
| `status.update` | status bar / activity feed | shelf |
| *(none equivalent)* | `ActivityNotice` | compression, approval |
| `process.list` RPC | `/tail` + `ProcessTailPanel` | 4096-char overlay |

---

## Code anchors

**Hermes — tool.generating → trail**

```668:673:../hermes-agent/ui-tui/src/app/createGatewayEventHandler.ts
      case 'tool.generating':
        if (ev.payload?.name) {
          turnController.pushTrail(`drafting ${ev.payload.name}…`)
        }
        return
```

**EdgeCrab — ToolGenerating emit**

```3487:3491:../../crates/edgecrab-core/src/conversation.rs
                        let _ = event_tx.send(crate::StreamEvent::ToolGenerating {
                            tool_call_id: tool_id,
                            name,
                            partial_args: entry.arguments.clone(),
                        });
```

**EdgeCrab — progress throttle constants**

```13:18:../../crates/edgecrab-tools/src/tool_progress_tail.rs
pub const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(200);
pub const OUTPUT_TAIL_LINE_COUNT: usize = 3;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 2;
```

**Hermes — foreground wait (no stdout to UI)**

```55:78:../hermes-agent/tools/environments/base.py
def touch_activity_if_due(
    state: dict,
    label: str,
) -> None:
    """Fire the activity callback at most once every ``state['interval']`` seconds.
    ...
    interval = state.get("interval", 10.0)
```

```687:688:../hermes-agent/tools/environments/base.py
                # Periodic activity touch so the gateway knows we're alive
                touch_activity_if_due(_activity_state, "terminal command running")
```

**Hermes — process.list 4KB tail**

```7567:7574:../hermes-agent/tui_gateway/server.py
        # The 200-char list preview is too thin for the desktop's inline
        # terminal viewer — ship a real tail alongside it.
        entry["output_tail"] = (proc.output_buffer or "")[-4000:]
        owned.append(entry)
    return owned


@method("process.list")
```

**EdgeCrab — `/tail` 4KB**

```1:3:../../crates/edgecrab-cli/src/process_tail_panel.rs
//! `/tail` overlay — read-only view of a background process buffer (Hermes `process.list` parity).

pub const TAIL_PANEL_MAX_CHARS: usize = 4096;
```
