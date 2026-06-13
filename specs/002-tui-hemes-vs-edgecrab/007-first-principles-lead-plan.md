# 007 — First Principles Plan: EdgeCrab Leads on TUI Overall

**Status:** Phase 1 + partial Phase 0/2/3 **shipped** (June 2026 pass)

---

## Completed ✅

| Phase | Item | Evidence |
|-------|------|----------|
| **1** | `/agents` overlay | `agents_overlay.rs`, `/agents` slash command |
| **1** | STOP steer from overlay | `i` key → `SteeringKind::Stop` |
| **1** | Shelf `/agents` hint | `activity_shelf.rs` |
| **0** | `stream_bridge.rs` + tests | 5 unit tests; wired in `check_responses` |
| **0** | `model_catalog_ui.rs` | model selector data layer extracted |
| **0** | `status_chrome.rs` | status bar spinner strings extracted |
| **0** | `overlay_layout.rs` picker DRY | `picker_three_layout`, `picker_help_line` |
| **2** | Reasoning auto-expand | `effective_thinking_render` — no `/details` mutation |
| **1b** | Spawn token/cost diff | `TurnCommitMetrics` on `commit_turn` ✅ |
| **0** | Status bar extraction | `display_state.rs`, `status_summaries.rs`, `status_bar.rs` ✅ |
| **0** | Response dispatch | `app/response_dispatch.rs` ✅ |
| **0** | Stream forward | `app/stream_forward.rs` ✅ |
| **0** | Steering overlay | `app/steering_overlay.rs` ✅ |
| **0** | Approval overlay | `approval_overlay.rs` + `app/approval_overlay.rs` ✅ |
| **0** | Value-capture overlay | `value_capture_overlay.rs` + `app/value_capture_overlay.rs` ✅ |
| **0** | Secret-capture overlay | `secret_capture_overlay.rs` + `app/secret_capture_overlay.rs` ✅ |
| **0** | Mode selector overlays | `app/mode_selectors.rs` (6 pickers) ✅ |
| **0** | Model selector overlays | `app/model_selectors.rs` ✅ |
| **0** | Browser selector overlays | `app/browser_selectors.rs` ✅ |
| **0** | Log/session browser overlays | `app/log_session_browsers.rs` ✅ |
| **0** | Gateway diagnose overlay | `app/diagnose_overlay.rs` ✅ |
| **0** | Browser chrome DRY | `app/browser_chrome.rs` + `overlay_layout.rs` ✅ |
| **0** | Setup overlays | `app/setup_overlays.rs` ✅ |
| **0** | Frame render stack | `app/frame_render.rs` ✅ |
| **0** | Input panel | `app/input_panel.rs` ✅ |
| **0** | Stream dispatch harness | `stream_dispatch_harness.rs` (Hermes turnController tests) ✅ |
| **0** | Picker chrome DRY | `picker_chrome.rs` (`selector_marker`) ✅ |
| **0** | Side-by-side spawn diff | `agents_overlay.rs` DiffView panes ✅ |
| **3** | Transcript render module | `transcript.rs` ✅ |
| **3** | Verbose trail 800 cap | `transcript_heights::truncate_verbose_trail` |
| **3** | Height cache + 800 cap | `transcript_heights.rs` + `transcript_scroll.rs` |
| **3** | Verbose trail dual cap | `VERBOSE_TRAIL_MAX_LINES=12` + char cap (Hermes parity) ✅ |
| **3b** | `/model` instant hot-swap | `switch_model_fast` (Hermes `config.set`); `/transfer-model` keeps brief ✅ |
| **3c** | Parallel tools expanded shelf | `SHELF_MAX_TOOL_ROWS_FULL=12` + drafting row ✅ |
| **3d** | Live tool-arg streaming | `bounded_live_render_text` (512-char shelf budget) ✅ |
| **3f** | Transcript height budget | `MAX_ESTIMATE_LINES=800` + char-budget bail in `estimate_wrapped_lines_capped` ✅ |
| **3g** | Expensive model guard | `model_cost_guard.rs` + picker `ExpensiveConfirm` overlay ✅ |
| **3h** | Subagent depth tree | `subagent_tree.rs` + `SubAgentStart { agent_id, parent_id }` + `/replay` ✅ |
| **3m** | Queued messages panel | `queued_messages.rs` + composer strip ✅ |
| **3n** | Queue edit UX | `app/queue_edit.rs` — Esc/Ctrl+X/↑↓ + commit on Enter ✅ |
| **3p** | Disk spawn-tree persistence | `spawn_tree_store.rs` — save on commit + `/replay list|load` ✅ |
| **3r** | Per-subagent interrupt | `subagent_registry.rs` + `/agents` `x`/`X` ✅ |
| **3i** | SpawnHud cap warnings | `spawn_hud.rs` + status bar wiring ✅ |
| **3j** | Gantt delegate timeline | `gantt_strip.rs` in `/agents` ✅ |
| **3k** | `/indicator` status styles | `status_indicator.rs` + config persist ✅ |
| **3l** | Mistral tool-call round-trip | `edgequake-llm` `convert_messages` preserves `tool_calls` ✅ |
| **3q** | Global spawn pause | `delegation_state.rs` + `/agents` `p` + status bar chip ✅ |

---

## In progress / next

| Phase | Item | Blocker |
|-------|------|---------|
| **0** | `check_responses` extraction | `app/response_dispatch.rs` ✅ |
| **0** | Stream forward bridge | `app/stream_forward.rs` ✅ |
| **0** | Steering overlay | `app/steering_overlay.rs` ✅ |
| **0** | Model selector overlays | `app/model_selectors.rs` ✅ |
| **0** | Log/session browser overlays | `app/log_session_browsers.rs` ✅ |
| **0** | Browser chrome DRY | `app/browser_chrome.rs` + `overlay_layout.rs` ✅ |
| **0** | Frame render stack | `app/frame_render.rs` ✅ |
| **0** | Input panel | `app/input_panel.rs` ✅ |
| **0** | `app/event_loop.rs` | poll/draw loop extracted from `app.rs` ✅ |
| **0** | Queued messages panel | `queued_messages.rs` — Hermes `QueuedMessages` above composer ✅ |
| **0** | Queue edit UX | `app/queue_edit.rs` — Hermes `cycleQueue` + Esc/Ctrl+X ✅ |
| **0** | Key dispatch module | `app/key_dispatch.rs` — `handle_key_event` extracted ✅ |
| **0** | `app.rs` < 5k lines | ~**24.0k** core — continue overlay/handler extraction |
| **4** | Model picker disconnect UI | **shipped** — `model_picker.rs` + `auth_cmd::disconnect_catalog_provider` |

---

## Definition of “EdgeCrab leads on TUI” — checklist

- [x] `/agents` ships with interrupt path (STOP steer)
- [x] Anti-stuck rows ≥ A− (live tail preserved)
- [x] Matrix row 8 ≥ A− (was F)
- [x] Clarify abandon flush (Hermes parity)
- [x] Spawn diff MVP (`d` in `/agents`)
- [x] Transcript MAX_HISTORY 800 wired
- [x] Model selector status hints (filter match / current model)
- [x] Reasoning auto-expand on live COT snippet
- [x] Model picker Ctrl+D disconnect (Hermes ^d parity)
- [x] Spawn diff token/cost metrics (Hermes Δ tokens/cost parity at turn level)
- [x] Approval overlay pure key dispatch (Hermes `approvalAction` parity + 1–4 keys)
- [x] Value-capture overlay pure key dispatch + masked display helpers
- [x] Secret-capture overlay (Hermes `MaskedPrompt` parity)
- [x] Stream bridge tail-3 integration test (`tool_progress_applies_tail_three_detail`)
- [x] Stream dispatch harness (terminal build + delegation lifecycle tests)
- [x] Verbose trail dual cap (800 chars + 12 lines, Hermes `limits.ts`)
- [x] Stream bridge tests ≥ 5 (now **7** incl. `extract_streaming_section`)
- [x] `/model` instant hot-swap (Hermes `config.set` parity; `/transfer-model` keeps brief)
- [x] Parallel tools visible when `/details tools expanded` (up to 12 rows)
- [x] Live tool-arg streaming on shelf (`bounded_live_render_text`)
- [x] Stream dispatch harness **6** tests (parallel tools + generating stream)
- [x] Expensive-model confirm before `/model` switch (Hermes cost guard parity)
- [x] Transcript wrap estimate byte budget (`MAX_ESTIMATE_LINES=800`)
- [x] Subagent `agent_id` + `parent_id` on `SubAgentStart` + parent-aware tree sort in `/agents`
- [x] In-memory `/replay` + overlay `[`/`]` history scrub
- [x] Queued messages panel above composer (`/queue` visibility)
- [x] Queue edit UX — Esc cancel, Ctrl+X delete, ↑↓ cycle (`app/queue_edit.rs`)
- [x] Key dispatch extracted to `app/key_dispatch.rs` (~2.7k lines)
- [x] Disk spawn-tree persistence + `/replay load` (`spawn_tree_store.rs`)
- [x] Per-subagent interrupt from `/agents` (`x` kill · `X` subtree)
- [x] Global spawn pause from `/agents` (`p`) + `delegate_task` reject when paused
- [x] Event loop extracted to `app/event_loop.rs`
- [x] SpawnHud depth/concurrency cap warnings in status bar
- [x] Gantt timeline strip in `/agents` overlay (≥2 delegates)
- [x] `/indicator` hot-swap (kaomoji / emoji / unicode / ascii)
- [ ] `app.rs` < 5k lines (~24k core remaining)
- [ ] Product rows 16–21 all ≥ Hermes (row 20 UI architecture **B** vs Hermes **A−**)

**Current claim:** **Liveness leadership + delegation control/replay parity with Hermes (incl. spawn pause).** Full product leadership still blocked by `app.rs` Phase 0 (~23.9k core). Hermes-only: plugins hub, FPS pane, provider-stage model wizard.

---

## Priority stack (updated)

1. ~~`/agents` overlay~~ ✅  
2. ~~`stream_bridge` + tests~~ ✅  
3. ~~Extract `check_responses` → `response_dispatch`~~ ✅  
4. Extract overlay bundle — steering ✅ · approval ✅ · value-capture ✅  
5. ~~Extract `render_output` → `transcript.rs`~~ ✅  
6. ~~Spawn **token/cost** diff viewer~~ ✅  
7. ~~Spawn history~~ ✅  
8. ~~Clarify abandon~~ ✅  
9. ~~Spawn metric diff~~ ✅  

---

## Non-negotiable invariants (unchanged)

> **Liveness beats lifecycle-only** — never drop `ToolProgress` tail.

> **Separate live from history** — shelf ephemeral, transcript durable.

See [002-progress-and-liveness.md](002-progress-and-liveness.md).
