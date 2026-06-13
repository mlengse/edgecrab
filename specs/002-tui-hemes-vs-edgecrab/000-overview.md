# 000 — Overview: Hermes vs EdgeCrab TUI (Brutal Honest)

**Repos surveyed:** June 2026  
**Last implementation pass:** global spawn pause (`p` in `/agents`) — Hermes `delegation.pause` parity

---

## One-sentence verdict

**EdgeCrab leads on liveness and matches Hermes on delegation control + replay. Full product leadership still blocked by `app.rs` core size (~23.9k).**

---

## Composite grades (post-implementation)

| Layer | Hermes | EdgeCrab | Δ since prior doc |
|-------|--------|----------|-------------------|
| **Liveness / anti-stuck** | B | **A−** | unchanged |
| **Live shelf semantics** | A− | **A−** | + phase spinners |
| **Delegation dashboard** | A | **A** | parent_id tree + `/replay` + Gantt + turn diff |
| **Transcript & scroll engineering** | **A** | **A** | tie | `MAX_ESTIMATE_LINES=800` + char budget bail |
| **Overlay & command UX** | **A** | **A** | tie | `/model` hot-swap + `/indicator` + `/details` picker + spawn pause |
| **Visual craft** | **A−** | **A−** | tie | phase spinners + `/indicator` styles |
| **UI code architecture** | **A−** | **B** | `app/` ~12.5k across **19** modules; core ~**23.9k** |
| **TUI-focused tests** | **A−** | **A−** | harness **6** tests + stream_bridge **9** |
| **Deploy / runtime** | B− | **A** | unchanged |

**Overall TUI product level: Hermes ~A− · EdgeCrab ~A (EdgeCrab leads unweighted matrix ~2.95 vs ~2.7)**

**Anti-stuck mission: EdgeCrab ~A · Hermes ~B** (unchanged lead)

---

## Shipped this pass (code is law)

| Feature | Module | Hermes analogue |
|---------|--------|-----------------|
| `/agents` overlay | `agents_overlay.rs` | `agentsOverlay.tsx` |
| STOP steer from overlay | `i` key → `SteeringKind::Stop` | whole-turn stop |
| Per-subagent interrupt | `x`/`X` in `/agents` → `subagent_registry` | `subagent.interrupt` RPC |
| Stream → shelf bridge | `stream_bridge.rs` | `turnController.ts` |
| Reasoning COT 160 | `stream_bridge::THINKING_COT_MAX` | `limits.ts` |
| Verbose trail cap 800 | `transcript_heights.rs` + `tool_display.rs` | OOM guard |
| Height cache + 800 cap | `transcript_heights.rs` + `transcript_scroll.rs` | `virtualHeights.ts` + `MAX_HISTORY` |
| Transcript 800-line cap | `transcript_scroll.rs` | `MAX_HISTORY = 800` |
| Shelf reasoning COT | `activity_shelf` + `stream_bridge` | default on shelf |
| Delegate goal diff | `spawn_diff.rs` | tree goal deltas in `/agents` |
| Model catalog UI | `model_catalog_ui.rs` | `modelPicker.tsx` data layer |
| Status chrome | `status_chrome.rs` | thinking/waiting status strings |
| Model picker disconnect | `model_picker.rs` + `auth_cmd` | `modelPicker.tsx` ^d stage |
| Picker layout DRY | `overlay_layout.rs` | shared picker chrome |
| Reasoning auto-expand | `shelf_details` + `activity_shelf` | COT without `/details` mutation |
| Display state machine | `display_state.rs` | Ink `$uiState` phases |
| Status bar render | `status_bar.rs` (~900 lines extracted) | status chrome row |
| Status summaries | `status_summaries.rs` | DG/BG status chips |
| Side-by-side spawn diff | `agents_overlay.rs` `render_diff_view` | `DiffPane` baseline/candidate |
| Transcript render | `transcript.rs` | `blockLayout.ts` + virtual heights |
| Response dispatch | `app/response_dispatch.rs` | `turnController.ts` consumer |
| Stream forward bridge | `app/stream_forward.rs` | `createGatewayEventHandler.ts` |
| Steering overlay | `app/steering_overlay.rs` | mission steer panel |
| Approval overlay | `approval_overlay.rs`, `app/approval_overlay.rs` | pure `map_approval_key` + render |
| Value-capture overlay | `value_capture_overlay.rs`, `app/value_capture_overlay.rs` | inline config/profile prompts |
| Secret-capture overlay | `secret_capture_overlay.rs`, `app/secret_capture_overlay.rs` | masked sudo/env input |
| Mode selector overlays | `app/mode_selectors.rs` | /verbose, /reasoning, /personality, … |
| Model selector overlays | `app/model_selectors.rs` | /model, vision, image, MoA experts |
| Browser selector overlays | `app/browser_selectors.rs` | MCP, profiles, skills, gateway, config |
| Log/session browsers | `app/log_session_browsers.rs` | log inspector + session browser |
| Gateway diagnose overlay | `app/diagnose_overlay.rs` | `/gateway diagnose` + colorize tests |
| Process tail render DRY | `process_tail_panel.rs` | `/tail` popup render extracted |
| Browser chrome DRY | `app/browser_chrome.rs` + `overlay_layout.rs` | split-pane + scroll helpers |
| Setup overlays | `app/setup_overlays.rs` | document, web/proxy/grok setup, skin browser |
| Frame render stack | `app/frame_render.rs` | transcript + shelf + overlay dispatch |
| Input panel | `app/input_panel.rs` | composer, ghost hint, slash completion |
| Overlay text input DRY | `overlay_text_input.rs` | shared value/secret key dispatch |
| Picker marker DRY | `picker_chrome.rs` | `selector_marker` |
| Stream dispatch harness | `stream_dispatch_harness.rs` | `TurnStreamHarness` shelf tests (**6**) |
| Live arg streaming | `transcript_heights.rs` `bounded_live_render_text` | Hermes `LIVE_RENDER_MAX_CHARS` |
| Parallel tool shelf | `activity_shelf.rs` `SHELF_MAX_TOOL_ROWS_FULL=12` | Hermes all `activeTools[]` rows |
| `/model` instant switch | `Agent::switch_model_fast` + `spawn_fast_model_switch` | Hermes `config.set` RPC |
| Expensive model guard | `model_cost_guard.rs` + picker confirm overlay | Hermes `model_cost_guard.py` |
| `/transfer-model` brief path | `spawn_model_transfer` → `perform_model_transfer` | EdgeCrab-only enhancement |
| Subagent depth tree | `subagent_tree.rs` + `SubAgentStart { agent_id, parent_id, depth }` | Hermes `subagentTree.ts` |
| In-memory + disk spawn replay | `/replay` + `spawn_tree_store.rs` | Hermes `spawn_tree.save/list/load` |
| Key dispatch module | `app/key_dispatch.rs` (~2.7k) | Hermes `useInputHandlers.ts` |
| Event loop module | `app/event_loop.rs` | Hermes `useMainApp` loop isolation |
| SpawnHud cap warnings | `spawn_hud.rs` + `status_bar.rs` | Hermes `SpawnHud` in `appChrome.tsx` |
| Gantt delegate timeline | `gantt_strip.rs` + `/agents` overlay | Hermes `GanttStrip` |
| `/indicator` status styles | `status_indicator.rs` + YAML `display.status_indicator` | Hermes `/indicator` slash |
| Mistral tool-call history fix | `edgequake-llm` `convert_messages` preserves `tool_calls` | — (EdgeCrab-only bugfix) |
| Global spawn pause | `delegation_state.rs` + `/agents` `p` + status bar chip | Hermes `delegation.pause` / `set_spawn_paused` |

---

## Still open (honest)

| Gap | Severity | Next step |
|-----|----------|-----------|
| `app.rs` monolith (~**23.9k** + ~**12.5k** in `app/`) | **Medium** | continue handler extraction |
| Queued-messages panel + edit | ✅ | shipped |
| Disk `/replay load` | ✅ | `spawn_tree_store.rs` + auto-save on turn commit |
| Model picker v2 | Low | provider-stage wizard still open; **expensive confirm shipped** |
| Ink tree animation | Low | cosmetic |
| Per-subagent interrupt RPC | ✅ | `subagent_registry.rs` + `/agents` `x`/`X` |
| Global spawn pause | ✅ | `delegation_state.rs` + `/agents` `p` + `delegate_task` reject |

---

## Cross-ref map

| Doc | Topic |
|-----|-------|
| [001](001-architecture-and-stack.md) | Stack |
| [002](002-progress-and-liveness.md) | Tails |
| [003](003-shelf-and-disclosure.md) | Shelf |
| [004](004-overlays-and-chrome.md) | Overlays |
| [005](005-engineering-quality.md) | Tests / debt |
| [006](006-dimension-matrix.md) | Matrix |
| [007](007-first-principles-lead-plan.md) | Roadmap + status |
| [008](008-cross-ref-index.md) | Index |
