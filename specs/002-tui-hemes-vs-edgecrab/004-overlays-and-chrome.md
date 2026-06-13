# 004 — Overlays & Chrome

**First principle:** Agency requires **discoverable controls** — not buried slash documentation.

**Last updated:** frame render + input panel extraction (June 2026)

---

## Hermes leads (remaining gaps)

| Feature | Hermes | EdgeCrab |
|---------|--------|----------|
| **Model picker overlay** | `modelPicker.tsx` — fuzzy + provider disconnect + expensive confirm | **`model_picker.rs`** — fuzzy + Ctrl+D + **Y/N expensive confirm**; provider-stage wizard still open |
| **Plugins hub** | `pluginsHub.tsx` — toggle bundled plugins in overlay | `/plugins` command path |
| **Active session switcher** | `activeSessionSwitcher.tsx` | `/session` commands |
| **Spawn diff (full tree)** | side-by-side tree totals + token/cost metrics | **turn diff** — side-by-side panes + tokens/cost (`d`) + **Gantt timeline strip** |
| **Credits / dev HUD** | `appChromeStatusRuleDevCredits.test.tsx` | none |
| **FPS / perf pane** | `fpsOverlay.tsx`, `perfPane.tsx` | none |
| **Per-subagent interrupt** | pause / kill single delegate | **`x`/`X`** + `subagent_registry` (local, no RPC) |
| **Queued messages panel** | `queuedMessages.tsx` + edit mode | **`queued_messages.rs`** + **`app/queue_edit.rs`** (Esc/Ctrl+X/↑↓) |
| **SpawnHud cap warnings** | `SpawnHud` in status bar | **`spawn_hud.rs`** — depth/concurrency warn colors |
| **Status indicator styles** | `/indicator` kaomoji/emoji/unicode/ascii | **`status_indicator.rs`** + YAML persist |

**Chrome grade: Hermes A · EdgeCrab A** (SpawnHud + Gantt + disk `/replay` + per-subagent kill + spawn pause `p`)

---

## EdgeCrab leads or matches

| Feature | EdgeCrab | Hermes |
|---------|----------|--------|
| **`/agents` delegate monitor** | sort, **`x`/`X` kill**, STOP steer, **`p` spawn pause**, **`/replay` + disk load**, history, diff, **Gantt** | full tree + spawn pause `p` |
| **Mission steering overlay** | Ctrl+S HINT / REDIRECT / STOP | less prominent TUI chrome |
| **`/tail` process panel** | `process_tail_panel.rs` — 4096 chars | `process.list` — 4000 chars |
| **Skin engine** | `skin_engine.rs` YAML at startup | gateway skin RPC |
| **`/details` interactive picker** | `details_panel.rs` + YAML | RPC-only slash |
| **`/indicator` hot-swap** | gateway RPC | **`status_indicator.rs`** local YAML |
| **SpawnHud delegation caps** | status bar warn/error colors | **`spawn_hud.rs`** |
| **Clarify abandon persist** | `clarify_panel.rs` wired | `flushAbandonedClarify` |
| **Agents nudge** | `maybe_agents_nudge` once/turn | `maybeNudgeAgents` |
| **Phase-specific shelf spinners** | `tui_spinner.rs` THINK/TOOL/DELEGATE | `unicode-animations` npm |

---

## Composer & input

**Hermes** — heavy investment in `textInput.tsx` + scroll acceleration tests.

**EdgeCrab** — `app/input_panel.rs`: waiting-state spinner sync (FP53), slash validation border, Fish ghost hint, Hermes-style completion overlay with `selector_marker` DRY.

---

## Transcript rendering

**Hermes:** `virtualHeights.ts` + `blockLayout.ts`, `MAX_HISTORY = 800`.

**EdgeCrab:** `transcript.rs` (rich + compact render, ghost lines, role bars) + `transcript_heights.rs` + `transcript_scroll.rs` — 800-line cap + height cache.

**Grade: Hermes A · EdgeCrab A−** (architecture extracted; virtual-height layer still simpler than Ink)

---

## Visual animation (re-assessed)

EdgeCrab shelf now uses **phase-specific spinners** via `tui_spinner.rs`:

| Phase | Glyph set |
|-------|-----------|
| Thinking | braille cycle |
| Tool exec | bar ramp ▁→█ |
| Delegates | orbit ◐◓◑◒ |
| Clarify | ❓ pulse |

Gap vs Hermes is **variety count** (7+ named unicode-animations), not absence of motion.

**Grade: Hermes A− · EdgeCrab A−** (was B+)

---

## Brutal summary

Delegation UX is **parity** for control + replay + spawn pause. Remaining hole: **`app.rs` core size**.
