# 008 — Cross-Reference Index

Master navigation for [specs/002-tui-hemes-vs-edgecrab/](.) and linked code.

---

## This spec set

| ID | File | Topic |
|----|------|-------|
| 000 | [000-overview.md](000-overview.md) | Verdict, composite grades |
| 001 | [001-architecture-and-stack.md](001-architecture-and-stack.md) | Stack, process model, file sizes |
| 002 | [002-progress-and-liveness.md](002-progress-and-liveness.md) | Tails, heartbeats, gateway |
| 003 | [003-shelf-and-disclosure.md](003-shelf-and-disclosure.md) | Shelf, `/details` |
| 004 | [004-overlays-and-chrome.md](004-overlays-and-chrome.md) | Overlays, composer, scroll |
| 005 | [005-engineering-quality.md](005-engineering-quality.md) | Tests, perf, debt |
| 006 | [006-dimension-matrix.md](006-dimension-matrix.md) | Scored matrix |
| 007 | [007-first-principles-lead-plan.md](007-first-principles-lead-plan.md) | Plan to lead overall |
| 008 | [008-cross-ref-index.md](008-cross-ref-index.md) | This file |

---

## EdgeCrab related specs

| ID | Path | Notes |
|----|------|-------|
| 005 | [../002-terminal-ux-ui/005-honest-assessment.md](../002-terminal-ux-ui/005-honest-assessment.md) | Optimistic “ship it” |
| 009 | [../002-terminal-ux-ui/009-hermes-comparison.md](../002-terminal-ux-ui/009-hermes-comparison.md) | Prior comparison — superseded for grades |
| 010 | [../002-terminal-ux-ui/010-delightful-tui-plan.md](../002-terminal-ux-ui/010-delightful-tui-plan.md) | Shelf architecture plan |
| 004 | [../002-terminal-ux-ui/004-stream-event-contract.md](../002-terminal-ux-ui/004-stream-event-contract.md) | StreamEvent catalog |
| 006 | [../002-terminal-ux-ui/006-stuck-scenarios-playbook.md](../002-terminal-ux-ui/006-stuck-scenarios-playbook.md) | Debug scenarios |
| 007 | [../002-terminal-ux-ui/007-implementation-roadmap.md](../002-terminal-ux-ui/007-implementation-roadmap.md) | Prior fix roadmap |

---

## Hermes code anchors

| Symbol / topic | Path | Lines / notes |
|----------------|------|---------------|
| Thinking shelf | `hermes-agent/ui-tui/src/components/thinking.tsx` | 1,224 |
| Agents overlay | `hermes-agent/ui-tui/src/components/agentsOverlay.tsx` | 1,073 |
| Turn controller | `hermes-agent/ui-tui/src/app/turnController.ts` | 1,009 |
| Gateway event handler | `hermes-agent/ui-tui/src/app/createGatewayEventHandler.ts` | `tool.generating` ~L668 |
| Long-run charms | `hermes-agent/ui-tui/src/app/useLongRunToolCharms.ts` | 8s / 10s / 2-cap |
| Virtual heights | `hermes-agent/ui-tui/src/lib/virtualHeights.ts` | scroll perf |
| Block layout | `hermes-agent/ui-tui/src/domain/blockLayout.ts` | transcript |
| Limits / OOM | `hermes-agent/ui-tui/src/config/limits.ts` | `VERBOSE_TRAIL_MAX_CHARS=800` |
| Shelf merge | `hermes-agent/ui-tui/src/lib/liveProgress.ts` | `appendToolShelfMessage` |
| Subagent tree | `hermes-agent/ui-tui/src/lib/subagentTree.ts` | heat, sparkline |
| Model picker | `hermes-agent/ui-tui/src/components/modelPicker.tsx` | |
| Plugins hub | `hermes-agent/ui-tui/src/components/pluginsHub.tsx` | |
| Overlay store | `hermes-agent/ui-tui/src/app/overlayStore.ts` | |
| TUI gateway | `hermes-agent/tui_gateway/server.py` | 10,185 |
| `process.list` 4KB | `hermes-agent/tui_gateway/server.py` | ~L7567–7574 |
| `details_mode` RPC | `hermes-agent/tui_gateway/server.py` | ~L7147+ |
| Foreground wait | `hermes-agent/tools/environments/base.py` | `touch_activity_if_due` L55–78, L687 |
| Bg watcher 500 | `hermes-agent/gateway/run.py` | ~L12097–12104 |
| flushAbandonedClarify | `hermes-agent/ui-tui/src/app/createGatewayEventHandler.ts` | ~L104 |
| maybeNudgeAgents | `hermes-agent/ui-tui/src/app/createGatewayEventHandler.ts` | ~L199 |
| Spawn history store | `hermes-agent/ui-tui/src/app/spawnHistoryStore.ts` | last 8 snapshots |

---

## EdgeCrab code anchors

| Symbol / topic | Path | Lines / notes |
|----------------|------|---------------|
| TUI monolith | `crates/edgecrab-cli/src/app.rs` | **~23,902** core |
| Spawn tree store | `crates/edgecrab-cli/src/spawn_tree_store.rs` | disk save/list/load (`~/.edgecrab/spawn-trees/`) |
| Subagent interrupt | `crates/edgecrab-core/src/subagent_registry.rs` | `/agents` `x`/`X` → `Agent::interrupt()` |
| Spawn pause | `crates/edgecrab-tools/src/delegation_state.rs` | `/agents` `p` · blocks new `delegate_task` |
| Replay command | `crates/edgecrab-cli/src/app/replay_command.rs` | `/replay list|load` |
| Key dispatch | `crates/edgecrab-cli/src/app/key_dispatch.rs` | `handle_key_event` (~2,732 lines) |
| Queue edit | `crates/edgecrab-cli/src/app/queue_edit.rs` | Hermes `cycleQueue` parity |
| Queued messages | `crates/edgecrab-cli/src/queued_messages.rs` | composer strip + edit highlight |
| Frame render | `crates/edgecrab-cli/src/app/frame_render.rs` | layout + overlay stack |
| Input panel | `crates/edgecrab-cli/src/app/input_panel.rs` | composer + completion |
| Browser chrome | `crates/edgecrab-cli/src/app/browser_chrome.rs` | split-pane detail + paging |
| Log/session browsers | `crates/edgecrab-cli/src/app/log_session_browsers.rs` | F5 / `/sessions` |
| Setup overlays | `crates/edgecrab-cli/src/app/setup_overlays.rs` | document, web/proxy/grok, skin |
| Diagnose overlay | `crates/edgecrab-cli/src/app/diagnose_overlay.rs` | `/gateway diagnose` |
| Mode selectors | `crates/edgecrab-cli/src/app/mode_selectors.rs` | 6 display pickers |
| Model selectors | `crates/edgecrab-cli/src/app/model_selectors.rs` | model/vision/image/MoA |
| Browser selectors | `crates/edgecrab-cli/src/app/browser_selectors.rs` | MCP/profile/skill/gateway/config |
| Stream harness | `crates/edgecrab-cli/src/stream_dispatch_harness.rs` | **6** turn lifecycle tests |
| Live arg streaming | `crates/edgecrab-cli/src/transcript_heights.rs` | `bounded_live_render_text`, `LIVE_RENDER_MAX_CHARS=512` |
| Transcript height budget | `crates/edgecrab-cli/src/transcript_heights.rs` | `MAX_ESTIMATE_LINES=800`, `estimate_wrapped_lines_capped` |
| Expensive model guard | `crates/edgecrab-core/src/model_cost_guard.rs` | Hermes `model_cost_guard.py` thresholds |
| Parallel tool shelf | `crates/edgecrab-cli/src/activity_shelf.rs` | `SHELF_MAX_TOOL_ROWS_FULL=12` |
| `/model` fast switch | `crates/edgecrab-core/src/agent.rs` | `switch_model_fast` |
| Model change CLI | `crates/edgecrab-cli/src/app.rs` | `spawn_fast_model_switch` / `spawn_model_transfer` |
| Picker chrome | `crates/edgecrab-cli/src/picker_chrome.rs` | `selector_marker` DRY |
| Overlay text input DRY | `crates/edgecrab-cli/src/overlay_text_input.rs` | shared value/secret key dispatch |
| Secret-capture UI | `crates/edgecrab-cli/src/app/secret_capture_overlay.rs` | masked sudo/env prompt |
| Approval key dispatch | `crates/edgecrab-cli/src/approval_overlay.rs` | `map_approval_key` + 7 tests |
| Approval UI | `crates/edgecrab-cli/src/app/approval_overlay.rs` | render + apply choice |
| Response dispatch | `crates/edgecrab-cli/src/app/response_dispatch.rs` | `check_responses` |
| Stream forward | `crates/edgecrab-cli/src/app/stream_forward.rs` | `forward_stream_event_to_tui` |
| Steering overlay | `crates/edgecrab-cli/src/app/steering_overlay.rs` | Ctrl+S mission steer panel |
| Transcript render | `crates/edgecrab-cli/src/transcript.rs` | `OutputLine`, rich/compact, ghost FP45 |
| Display state | `crates/edgecrab-cli/src/display_state.rs` | `DisplayState`, voice badges, context ratio |
| Status bar | `crates/edgecrab-cli/src/status_bar.rs` | ~909 lines extracted from `app.rs` |
| Status summaries | `crates/edgecrab-cli/src/status_summaries.rs` | DG/BG chip summaries |
| Activity shelf | `crates/edgecrab-cli/src/activity_shelf.rs` | 768 |
| Turn activity | `crates/edgecrab-cli/src/turn_activity.rs` | 776 |
| Shelf visuals | `crates/edgecrab-cli/src/shelf_visual.rs` | heat, sparkline |
| Shelf disclosure | `crates/edgecrab-cli/src/shelf_details.rs` | |
| `/details` panel | `crates/edgecrab-cli/src/details_panel.rs` | |
| Agents overlay | `crates/edgecrab-cli/src/agents_overlay.rs` | `/agents` + side-by-side diff (`d`) + depth-first sort |
| Subagent tree | `crates/edgecrab-cli/src/subagent_tree.rs` | Hermes `subagentTree.ts` — width, indent, sparkline |
| SubAgentStart ids | `crates/edgecrab-core/src/agent.rs` | `StreamEvent::SubAgentStart { agent_id, parent_id, depth }` |
| Subagent tree sort | `crates/edgecrab-cli/src/subagent_tree.rs` | `sort_tree_depth_first` |
| Spawn replay | `crates/edgecrab-cli/src/commands.rs` | `/replay [N\|last\|list]` |
| Queued messages | `crates/edgecrab-cli/src/queued_messages.rs` | composer strip above input |
| Event loop | `crates/edgecrab-cli/src/app/event_loop.rs` | poll/draw cadence |
| Spawn history | `crates/edgecrab-cli/src/spawn_history.rs` | `TurnCommitMetrics` on `commit_turn` |
| Spawn diff | `crates/edgecrab-cli/src/spawn_diff.rs` | tokens/cost/fan-out deltas |
| Model picker disconnect | `crates/edgecrab-cli/src/model_picker.rs` | Ctrl+D confirm stage |
| Model catalog UI | `crates/edgecrab-cli/src/model_catalog_ui.rs` | selector data + hints |
| Status chrome | `crates/edgecrab-cli/src/status_chrome.rs` | spinners, `format_token_count`, tool summary |
| Auth disconnect | `crates/edgecrab-cli/src/auth_cmd.rs` | `disconnect_catalog_provider` |
| Phase spinners | `crates/edgecrab-cli/src/tui_spinner.rs` | shelf phase glyphs |
| Clarify abandon | `crates/edgecrab-cli/src/clarify_panel.rs` | `format_abandoned_clarify` |
| Stream bridge | `crates/edgecrab-cli/src/stream_bridge.rs` | shelf apply_* + `maybe_agents_nudge` |
| Transcript heights | `crates/edgecrab-cli/src/transcript_heights.rs` | `VERBOSE_TRAIL_MAX_CHARS/LINES` |
| Overlay layout | `crates/edgecrab-cli/src/overlay_layout.rs` | `popup_rect` |
| Shelf coalesce | `crates/edgecrab-cli/src/live_progress.rs` | 16ms |
| Tool display | `crates/edgecrab-cli/src/tool_display.rs` | |
| Slash commands | `crates/edgecrab-cli/src/commands.rs` | `/details`, `/tail`, `/indicator` |
| SpawnHud | `crates/edgecrab-cli/src/spawn_hud.rs` | status bar cap warnings |
| Gantt strip | `crates/edgecrab-cli/src/gantt_strip.rs` | `/agents` timeline |
| Status indicator | `crates/edgecrab-cli/src/status_indicator.rs` | `/indicator` styles |
| Mistral tool history | `../edgequake-llm/src/providers/openai_compatible.rs` | `convert_messages` tool_calls |
| Theme / skin | `crates/edgecrab-cli/src/theme.rs`, `skin_engine.rs` | |
| Progress DRY | `crates/edgecrab-tools/src/tool_progress_tail.rs` | tail-3, 200ms |
| `ToolGenerating` | `crates/edgecrab-core/src/conversation.rs` | ~L3487 |
| `ActivityNotice` | `crates/edgecrab-core/src/agent.rs`, `conversation.rs` | |
| `StreamEvent` | `crates/edgecrab-core/src/agent.rs` | ~L1956+ |
| Gateway tails | `crates/edgecrab-gateway/src/event_processor.rs` | |
| Gateway config | `crates/edgecrab-gateway/src/config.rs` | `bg_tail_chars: 500` |
| Process table | `crates/edgecrab-tools/src/process_table.rs` | bg buffers |

---

## Verified metrics (June 2026)

| Metric | Value |
|--------|-------|
| `app.rs` lines | 26,635 (was 38,243 — **−30%**) |
| `app/` submodule lines | ~9,233 (15 modules) |
| Hermes ui-tui source files | 202 |
| Hermes ui-tui test files | 71 |
| Transcript scroll | `crates/edgecrab-cli/src/transcript_scroll.rs` | `MAX_TRANSCRIPT_LINES = 800` |
| Fuzzy overlays | `crates/edgecrab-cli/src/fuzzy_selector.rs` | model/skill/session pickers |
| EdgeCrab-cli `#[test]` count (all modules) | ~710+ |
| Hermes `process.list` tail | 4,000 chars |
| EdgeCrab `/tail` tail | 4,096 chars |
| Hermes activity touch interval | 10s default |
| EdgeCrab progress emit interval | 200ms |
| EdgeCrab `/agents` | `agents_overlay.rs` + `/agents` slash |

---

## Document graph

```text
000-overview
 ├── 001-architecture
 ├── 002-progress-liveness
 ├── 003-shelf-disclosure
 ├── 004-overlays-chrome
 ├── 005-engineering-quality
 ├── 006-dimension-matrix
 └── 007-lead-plan
      └── 008-index (this file)
```

---

## External repo path

Hermes checkout used for survey: `/Users/raphaelmansuy/Github/03-working/hermes-agent`
