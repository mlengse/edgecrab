# 003 — Shelf & Disclosure (`/details`)

**First principle:** Separate **live** from **history** — transcript is narrative; shelf is ephemeral turn state.

---

## Semantic parity (post shelf pass)

Both products implement:

| Feature | Hermes | EdgeCrab |
|---------|--------|----------|
| Global disclosure | `details_mode`: hidden / collapsed / expanded | `ShelfDetailsMode` in YAML |
| Per-section overrides | `details_mode.thinking`, `.tools`, … | `display.shelf_details.sections.*` |
| `tool.generating` phase | `tool.generating` event | `StreamEvent::ToolGenerating` |
| Elapsed heat + sparkline | `subagentTree.ts` — `hotnessBucket`, `sparkline` | `shelf_visual.rs` — `elapsed_heat`, `sparkline` |
| Long-run hints | `useLongRunToolCharms` | `turn_activity::tick_long_run_hints` |
| Token-ish labels | `fmtTokens`, `estimateTokensRough` | `format_tokens_label`, `estimate_tokens_rough` |
| Section chevrons | `▸` / `▾` in thinking UI | `section_chevron` in `shelf_visual.rs` |
| Sub-agent tool churn | `toolCount`, output tail | `tool_count`, `recent_tools` on `ShelfSubagentRow` |

**Verdict:** Disclosure **semantics** — **tie (A−)**. Craft and integration — Hermes still ahead.

---

## Module mapping

| Concern | Hermes | EdgeCrab |
|---------|--------|----------|
| Shelf renderer | `thinking.tsx` (1,224 lines) | `activity_shelf.rs` (768 lines) |
| Turn state | `turnController.ts` (1,009 lines) | `turn_activity.rs` (776 lines) |
| Disclosure policy | `domain/details.js` + RPC | `shelf_details.rs` |
| Interactive picker | slash → RPC `config.set details_mode.*` | **`details_panel.rs`** — ratatui overlay on bare `/details` |
| Coalesced redraw | `STREAM_BATCH_MS` (16ms) | `live_progress.rs` — `SHELF_COALESCE_MS = 16` |

---

## Where Hermes still feels better

1. **Spinner variety** — per-phase braille sets (`THINK`, `TOOL` arrays) via `unicode-animations` (`thinking.tsx` L41–42). EdgeCrab uses one 10-frame braille cycle when `animate_status_indicators` is true; static `◦` when false (`activity_shelf.rs` L679–684).

2. **Tree chrome** — `TreeRow`, `TreeNode`, box-drawing stems (`thinking.tsx` L60–150). EdgeCrab uses flat shelf lines with chevrons.

3. **Reasoning in shelf by default** — `reasoning.delta` feeds thinking section; CoT capped at `THINKING_COT_MAX = 160` (`limits.ts`). EdgeCrab: opt-in `/reasoning show`; ghost line when enabled.

4. **Shelf merged into transcript trail** — `appendToolShelfMessage` (`liveProgress.ts`) keeps shelf state visible while scrolling history. EdgeCrab: shelf is a **fixed band** between transcript and status bar (cleaner layout, different scroll behavior).

5. **`/agents` discovery** — shelf surfaces `(/agents to monitor)` when delegates appear (`thinking.tsx` ~L1076–1121). EdgeCrab: **no `/agents` command or overlay** (grep: zero matches in `edgecrab-cli`).

---

## Where EdgeCrab is better

1. **`/details` picker UX** — interactive overlay with live preview of effective vs default modes (`details_panel.rs`). Hermes persists via RPC from slash text (`slash/commands/core.ts`).

2. **Local YAML persistence** — no gateway round-trip for `display.shelf_details`.

3. **Live prompt caption** — `TurnActivityState::live_caption()` replaces generic “waiting…” during tool runs (input panel title in `app.rs`).

4. **Mid-run shell tail on tool rows** — shelf can show stdout preview from `ToolProgress`; Hermes tool rows show spinner + args, not foreground shell bytes.

5. **Shelf live backstop** — `minimum_shelf_lines()` ensures at least one line when sections hidden but tools active.

---

## EdgeCrab shelf limits

```24:39:../../crates/edgecrab-cli/src/turn_activity.rs
pub const SHELF_MAX_TOOL_ROWS: usize = 3;
pub const SHELF_BG_TAIL_CHARS: usize = 120;
pub const SHELF_ACTIVITY_FEED_MAX: usize = 4;
```

```24:24:../../crates/edgecrab-cli/src/activity_shelf.rs
const MAX_SHELF_LINES: u16 = 6;
```

Hermes thinking panel scales with virtualized transcript; EdgeCrab hard-caps shelf height for calm layout.

---

## Progressive disclosure ladder (both)

| Level | Surface | EdgeCrab trigger |
|-------|---------|------------------|
| L0 | Status bar | always |
| L1 | Activity shelf | `is_processing` + `display.activity_shelf` |
| L2 | Transcript placeholders | `ToolProgressMode` / `/verbose` |
| L3 | Expand | Ctrl+Shift+T |
| L4 | Process tail | `/tail <id>` |

Documented in [../002-terminal-ux-ui/010-delightful-tui-plan.md](../002-terminal-ux-ui/010-delightful-tui-plan.md).

---

## Correction vs [009](../002-terminal-ux-ui/009-hermes-comparison.md)

009 marks “Ink accordion polish — **Closed (static)**”. Re-assessment:

- Chevrons + heat + sparklines: **closed**
- Animated accordion / tree expand: **not closed** — ratatui has no CSS-style expand; Hermes Ink tree is materially richer

---

## Code anchors

| Behavior | Path |
|----------|------|
| EC shelf render | `edgecrab-cli/src/activity_shelf.rs` — `render_activity_shelf` |
| EC `/details` | `edgecrab-cli/src/details_panel.rs`, `commands.rs` L836–840 |
| EC live caption | `edgecrab-cli/src/turn_activity.rs` — `live_caption` |
| Hermes thinking | `hermes-agent/ui-tui/src/components/thinking.tsx` |
| Hermes details RPC | `hermes-agent/tui_gateway/server.py` ~L7147+ |
| Hermes shelf merge | `hermes-agent/ui-tui/src/lib/liveProgress.ts` |
