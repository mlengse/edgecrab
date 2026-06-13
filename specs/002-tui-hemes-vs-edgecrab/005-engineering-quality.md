# 005 — Engineering Quality

## Test coverage (re-assessed)

| Metric | Hermes `ui-tui` | EdgeCrab `edgecrab-cli` |
|--------|-----------------|-------------------------|
| UI-focused test **files** | **71** under `src/__tests__/` | shelf modules have inline `#[test]` blocks |
| Event handler isolation | `createGatewayEventHandler.test.ts` (~1100+ lines) | **none** — logic in `app.rs` |
| Turn / shelf unit tests | `turnControllerNotice.test.ts`, `subagentTree.test.ts`, … | `turn_activity.rs` **9 tests**, `activity_shelf.rs` **6 tests**, `shelf_visual.rs` **7 tests** |
| Slash dispatch tests | `slashParity.test.ts`, `createSlashHandler.test.ts` | `commands.rs` — **79** test functions (mostly dispatch strings) |
| Total `#[test]` in `edgecrab-cli/src` | — | **~762** (includes CLI args, auth, setup — not TUI render paths) |

**Verdict:** EdgeCrab has strong **CLI crate** test volume; stream→shelf harness now covers parallel tools + generating lifecycle (**6** tests). Hermes still leads on full gateway handler isolation (~1500 lines). **Grade: Hermes A− · EdgeCrab A−** (TUI-specific — was B)

---

## Performance & memory

### Hermes — battle-tested limits

`VERBOSE_TRAIL_MAX_CHARS = 800` (persisted trails) vs `LIVE_RENDER_MAX_CHARS = 16_000` (streaming) — explicit comment that huge persisted trails OOM’d Ink (`limits.ts` L6–17).

Supporting tooling: `memoryMonitor.ts` + tests, virtual height cache to avoid remeasuring every frame.

### EdgeCrab

- Rust ownership avoids Node render-tree OOM class
- **`app.rs` 23,902 lines** (+ **~12.5k** in `app/` submodules) — **37%** down from 38,243 baseline
- Transcript virtual-height layer matches Hermes `MAX_ESTIMATE_LINES=800` + char-budget bail

---

## Maintainability scorecard

| Principle | Hermes | EdgeCrab |
|-----------|--------|----------|
| Single responsibility | A− (~202 UI files) | **B** (`app/` **18** submodules ~12.4k; core ~24.0k) |
| DRY progress formatting | B (Python + TS + gateway) | **A** (`tool_progress_tail.rs`) |
| Open/closed for new overlays | A (`overlayStore.ts`) | **A−** (`browser_chrome`, `setup_overlays`, `overlay_layout`) |
| UI state inspectability | B (nanostores: `$uiState`, `$overlayState`) | C (fields on `App` struct) |
| Shelf module SRP | A− | **A−** (post-extraction) |

---

## Operational complexity

| | Hermes | EdgeCrab |
|---|--------|----------|
| `hermes --tui` | spawn Node + Python `tui_gateway` | `edgecrab` / `cargo run` |
| Failure modes | stdin EOF if Node OOM (#34095) | single process panic |
| Desktop / web PTY | shared gateway | CLI-first; gateway separate |
| Debug UI | limited Ink introspection | `tracing`, `/debug` |

**Tradeoff:** Hermes pays **process** complexity for product breadth; EdgeCrab pays **monolith** complexity inside one binary.

---

## What “lead on TUI” requires first

You cannot sustainably out-polish Hermes while **`app.rs` remains 38k lines**. Module extraction is **Phase 0**, not polish.

Suggested extraction targets (see [007](007-first-principles-lead-plan.md)):

| Module | ~lines to remove from `app.rs` |
|--------|-------------------------------|
| `status_chrome.rs` | status bar spinner strings |
| `model_catalog_ui.rs` | model selector data + hints |
| `overlay_layout.rs` | picker geometry DRY |
| `app/response_dispatch.rs` | `check_responses` (~1.2k lines) ✅ |
| `app/stream_forward.rs` | `forward_stream_event_to_tui` ✅ |
| `transcript.rs` | `OutputLine` + `render_transcript_*` ✅ |
| `status_bar.rs` | `render_status_bar` + compact bar ✅ |
| `overlay/` | details, tail, approval, value-capture |
| `app/steering_overlay.rs` | mission steer panel ✅ |
| `app/approval_overlay.rs` | approval render + handlers ✅ |
| `approval_overlay.rs` | pure key dispatch (Hermes `approvalAction`) ✅ |
| `value_capture_overlay.rs` | pure key dispatch + masked display ✅ |
| `app/mode_selectors.rs` | 6 display mode pickers ✅ |
| `app/model_selectors.rs` | model/vision/image/MoA pickers ✅ |
| `app/browser_selectors.rs` | MCP/profile/skill/gateway/config browsers ✅ |
| `app/log_session_browsers.rs` | log + session browser overlays ✅ |
| `app/diagnose_overlay.rs` | gateway diagnose + `colorize_diagnose_line` test ✅ |
| `app/browser_chrome.rs` | split-pane detail + paging DRY ✅ |
| `app/setup_overlays.rs` | document/web/proxy/grok/skin overlays ✅ |
| `app/frame_render.rs` | main frame + overlay stack ✅ |
| `app/input_panel.rs` | composer + completion overlay ✅ |
| `overlay_layout.rs` | `browser_*` layout helpers + scroll test ✅ |
| `stream_dispatch_harness.rs` | StreamEvent → shelf test harness ✅ |
| `picker_chrome.rs` | shared `selector_marker` ✅ |
| `app/secret_capture_overlay.rs` | masked sudo/env capture ✅ |
| `app_loop.rs` | thin `event_loop` | **`app/event_loop.rs`** ✅ |
| `app/key_dispatch.rs` | key routing | **`app/key_dispatch.rs`** ✅ (~2.7k lines; duplicate Ctrl+S removed) |
| `app/queue_edit.rs` | queue edit mode | **`app/queue_edit.rs`** ✅ (Hermes `cycleQueue`) |

**Exit gate:** `app.rs` < 5,000 lines + stream bridge tests.

---

## Code anchors

| Topic | Path |
|-------|------|
| Hermes OOM guard | `hermes-agent/ui-tui/src/config/limits.ts` |
| Hermes virtual heights | `hermes-agent/ui-tui/src/lib/virtualHeights.ts` |
| Hermes overlay store | `hermes-agent/ui-tui/src/app/overlayStore.ts` |
| `app/event_loop.rs` | poll/draw loop (was inline in `app.rs`) |
| `queued_messages.rs` | Hermes `QueuedMessages` composer strip |
| `app/queue_edit.rs` | queue edit: Esc cancel, Ctrl+X delete, ↑↓ cycle |
| `app/replay_command.rs` | `/replay list|load` handler |
| `spawn_tree_store.rs` | Hermes `spawn_tree.save/list/load` |
| `delegation_state.rs` | Hermes `delegate_tool.set_spawn_paused` |
| `app.rs` | monolith (~24.0k + ~12.4k in `app/`) |
| EC shelf tests | `activity_shelf.rs`, `turn_activity.rs` `#[cfg(test)]` |
| EC progress DRY | `edgecrab-tools/src/tool_progress_tail.rs` |
