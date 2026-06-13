# 005 — Honest Assessment (EdgeCrab TUI)

**Last updated:** spawn diff + phase spinners (June 2026)

## Overall grade: **A−** liveness · **A−** shelf · **A−** product TUI · **C+** architecture

Hermes parity is **closed for anti-stuck + shelf disclosure**. Delegation dashboard gap **closed (MVP)**. Full product leadership still blocked by `app.rs` size.

---

## Dimension scores

| Dimension | Grade | Verdict |
|-----------|-------|---------|
| Tool lifecycle | **A** | stream_bridge centralizes shelf updates |
| Tool mid-run detail | **A−** | Tail-3 preserved |
| UI chrome / shelf | **A−** | heat, sparklines, `/agents` hints |
| `/agents` dashboard | **A−** | overlay + sort + STOP + turn history + `d` diff |
| Visual animation | **A−** | phase spinners (`tui_spinner.rs`) |
| `/details` UX | **A** | picker + YAML persist |
| Sub-agent shelf | **A−** | tool churn + sparkline |
| Parallel tools | **A−** | ≤3 rows |
| Background visibility | **A−** | 4KB `/tail` |
| Transcript perf | **B−** | **A−** | 800 cap + height cache |
| Reasoning on shelf | B | **A−** | COT snippet without `/reasoning show` |
| Code architecture | **C+** | 6 new modules; `app.rs` still monolith |
| TUI unit tests | **B** | stream_bridge, agents_overlay, transcript_heights |

---

## Architecture (SOLID / DRY)

```text
StreamEvent → stream_bridge (pure apply_* fns + tests)
           → turn_activity (live map + live_caption)
           → shelf_details / shelf_visual
           → activity_shelf (render + /agents hints)
           → agents_overlay (/agents dashboard)
           → details_panel (/details picker)
           → transcript_heights (800 cap + height cache)
           → app.rs (event loop — still too large)
```

| Principle | Score |
|-----------|-------|
| Single responsibility | **B+** (improving) |
| DRY | **A** |
| Open/closed | **B** |

---

## Shipped this pass

1. **`agents_overlay.rs`** — `/agents` full-screen monitor (Hermes `agentsOverlay.tsx` MVP)
2. **`stream_bridge.rs`** — testable shelf mutations from stream events
3. **`transcript_heights.rs`** — verbose trail cap + line height cache
4. **`overlay_layout.rs`** — shared popup geometry
5. **Reasoning COT 160** — `THINKING_COT_MAX`
6. **Shelf hints** — `(/agents to monitor)` on delegate sections

---

## Remaining gaps (honest)

| Gap | Severity |
|-----|----------|
| `app.rs` ~38k lines | **High** |
| Spawn tree diff | Medium |
| Spawn metric diff | **Done** (`spawn_diff.rs`, `d` in `/agents`) |
| Model picker v2 | Medium |
| Animated Ink accordion | Low |
| Per-delegate interrupt (not whole STOP steer) | Medium |

---

## vs Hermes

| Area | EdgeCrab | Hermes |
|------|----------|--------|
| Live shell tail | **A−** | C |
| `/agents` dashboard | **A−** | A (tree diff, pause) |
| Visual craft | **A−** | A− |
| `/details` + persistence | **A** | B+ |
| Transcript scale | **A−** | A |
| UI modularity | **C+** | A− |

---

## Verdict

**Continue shipping.** EdgeCrab exceeds Hermes on liveness and matches shelf semantics. `/agents` closes the largest UX hole. Next blocker for “lead overall” is **`app.rs` extraction**, not more shelf polish.

---

## Cross-references

- [../002-tui-hemes-vs-edgecrab/000-overview.md](../002-tui-hemes-vs-edgecrab/000-overview.md)
- [../002-tui-hemes-vs-edgecrab/007-first-principles-lead-plan.md](../002-tui-hemes-vs-edgecrab/007-first-principles-lead-plan.md)
- [010-delightful-tui-plan.md](010-delightful-tui-plan.md)
