# 006 — Dimension Matrix (Code-Anchored)

Grades: **S** · **A** · **B** · **C** · **F**

Re-assessed June 2026 against both repos. “Δ” = leader on row.

| # | Dimension | Hermes | EC | Δ | Primary evidence |
|---|-----------|--------|----|---|------------------|
| 1 | Tool lifecycle start/done | A | A | tie | `tool.start` / `ToolExec`; `tool.complete` / `ToolDone` |
| 2 | Foreground stdout mid-run | **C** | **A−** | **EC** | `base.py` L687 vs `tool_progress_tail.rs` tail-3 |
| 3 | Tool arg streaming UX | A− | **A−** | tie | `bounded_live_render_text` + drafting row on shelf |
| 4 | Reasoning visibility | A− | **A** | **EC** | auto-expand COT on shelf without `/details` mutation |
| 5 | Activity shelf semantics | A− | A− | tie | `thinking.tsx` vs `activity_shelf.rs` |
| 6 | `/details` disclosure UX | B+ | **A−** | **EC** | RPC slash vs `details_panel.rs` + YAML |
| 7 | Sub-agent shelf rows | A− | **A−** | tie | sparkline + `/agents` nudge + **parent_id on start** |
| 8 | Sub-agent dashboard | **A** | **A** | tie | kill + STOP + disk `/replay` + Gantt |
| 9 | Long-run charms | A− | A− | tie | 8s / 10s / 2-per-tool both |
| 10 | Background tail panel | A | A | tie | 4000 vs 4096 chars |
| 11 | Gateway foreground tool progress | B | A− | EC | 1.5s edit throttle vs tail status |
| 12 | Gateway bg running push | A− | A− | tie | 500-char watcher vs `bg_tail_chars: 500` |
| 13 | Compression UX | C | **A−** | **EC** | `ActivityNotice` |
| 14 | Approval UX | A− | **A−** | tie | `approval_overlay.rs` pure dispatch + 1–4 keys |
| 15 | Parallel tools display | B | **A−** | **EC** | up to 12 rows when `/details tools expanded` |
| 16 | Verbose tool trails | A− | **A−** | tie | 800 chars + 12 lines + Ctrl+Shift+T expand |
| 17 | Transcript scroll perf | **A** | **A** | tie | `estimate_wrapped_lines_capped` + `MAX_ESTIMATE_LINES` |
| 18 | Visual animation craft | **A−** | **A−** | tie | `tui_spinner.rs` + `/indicator` kaomoji/emoji/unicode/ascii |
| 19 | Overlay ecosystem | **A** | **A** | tie | chrome + setup modules; `/indicator` + SpawnHud + spawn pause shipped |
| 20 | UI architecture | **A−** | **B** | **H** | `app/key_dispatch.rs` + `queue_edit.rs`; core ~24k |
| 21 | TUI-focused tests | **A−** | **A−** | tie | harness **6** + stream_bridge **9** |
| 22 | Runtime simplicity | B− | **A** | **EC** | multi-process vs binary |
| 23 | Progress DRY module | B | **A** | **EC** | scattered vs `tool_progress_tail.rs` |
| 24 | Config persistence (disclosure) | B+ | A− | EC | RPC vs local YAML |

---

## Score summary

Numeric map: S=4, A=3, B=2, C=1, F=0

| Lens | Hermes mean | EdgeCrab mean |
|------|-------------|---------------|
| Unweighted (all 24 rows) | **~2.7 (B+/A−)** | **~2.95 (A)** |
| **Anti-stuck weighted** (rows 2, 5, 9, 11, 12, 13 ×2) | ~2.5 | **~3.0** |
| **Product polish weighted** (rows 8, 16–21 ×2) | **~3.0** | **~3.1** |

---

## Headline corrections vs [009](../002-terminal-ux-ui/009-hermes-comparison.md)

| 009 claim | Re-assessment |
|-----------|---------------|
| EdgeCrab **A/A** overall visibility/liveness | **A− / A−** on liveness; **B+** on holistic TUI |
| “Ink accordion polish — Closed” | Chevrons closed; **tree animation not closed** |
| EdgeCrab ahead on all bg process | **Tie** on 4KB panel + 500-char gateway push |
| “Sub-agent tree — A− parity” | Shelf **A−**; dashboard **A− vs A** (spawn history MVP) |

---

## Scenario → grade quick lookup

| User question | Best product |
|---------------|--------------|
| “Is `cargo build` stuck?” | **EdgeCrab** |
| “What are my subagents doing?” | **Tie** (EC kill + Gantt + `/agents` + disk `/replay` + spawn pause `p`) |
| “How do I hide tool noise?” | **Tie** (`/details`) |
| “Will my TUI scale to 800 turns?” | **Tie** (both cap at 800 lines) |
| “Can I ship one binary to Termux?” | **EdgeCrab** |

---

## Code anchor index

See [008-cross-ref-index.md](008-cross-ref-index.md) for full symbol → file map.
