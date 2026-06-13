# 002 — Terminal & Agent Visibility Audit

**Status:** Audit complete · **P0–P3b implemented** on `feat/terminal-ux-live-progress` (June 2026)  
**Goal:** Understand exactly how EdgeCrab surfaces agent + terminal activity, and why users sometimes feel the agent is **stuck**.

## Problem statement

EdgeCrab’s TUI streams **throttled tail lines** during shell work (local, PTY, Docker live; remote batch on completion). Full stdout still goes to the LLM in tool results. Users no longer stare at a blank spinner for minutes, but this is **not** a full terminal mirror.

The agent never truly “hangs” in most cases — it is blocked on tool I/O — but observability must keep improving for web tools and gateway parity.

## Scope

| In scope | Out of scope |
|----------|--------------|
| Foreground `terminal` tool | Gateway platform-specific formatting (covered briefly) |
| Background `run_process` / `ProcessTable` | Non-CLI surfaces (ACP, SDK) except where they share `StreamEvent` |
| TUI `DisplayState` + status bar | Hermes parity → [010-delightful-tui-plan.md](010-delightful-tui-plan.md) |
| `StreamEvent` contract + `edgequake_llm` streaming | Full redesign mockups |
| Parallel tool dispatch visibility | |

## Document map

| Doc | Contents |
|-----|----------|
| [001-data-flow-map.md](001-data-flow-map.md) | End-to-end path: LLM → tool → stream → TUI |
| [002-terminal-and-process-tools.md](002-terminal-and-process-tools.md) | Shell execution, buffering, batch-only output |
| [003-tui-visibility-layer.md](003-tui-visibility-layer.md) | What the user actually sees; `DisplayState` machine |
| [004-stream-event-contract.md](004-stream-event-contract.md) | Every event variant: emitted? consumed? wired? |
| [005-honest-assessment.md](005-honest-assessment.md) | Brutal gap analysis + severity matrix |
| [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md) | Scenario → root cause → code anchor |
| [007-implementation-roadmap.md](007-implementation-roadmap.md) | Prioritized fixes with touch points |
| [008-cross-ref-index.md](008-cross-ref-index.md) | Master code index |
| [009-hermes-comparison.md](009-hermes-comparison.md) | **Hermes-agent TUI & progression comparison** |
| [010-delightful-tui-plan.md](010-delightful-tui-plan.md) | **First-principles plan: polished shelf + Hermes best ideas** |

## Executive verdict (post-implementation)

**What works well**

- **Live tail progress** via `tool_progress_tail.rs` — local shell, PTY, Docker (stream), remote batch (tail on complete).
- Tool lifecycle: `ToolExec` → spinner → `ToolDone` with duration and compact result preview.
- Background processes: `BackgroundProcessTail` / `BackgroundProcessFinished` monitor lines.
- Compression + steering: `ActivityNotice`, `SteerPending` / `SteerApplied`.
- Verbose-off: dim `⏳` indicator still updates in-place on `ToolProgress`.
- Cancellation propagates to tools; context pressure gauge on gateway.

**Remaining gaps**

1. **Gateway** — status snippets, not in-place TUI monitor lines; Hermes sends longer bg-process pushes (see [009-hermes-comparison.md](009-hermes-comparison.md)).
2. **UI chrome** — Hermes Ink “thinking shelf” and desktop process viewer are richer than ratatui inline lines.
3. **True terminal scrollback** — by design in both products; tail-3 + expand is the model.

See [005-honest-assessment.md](005-honest-assessment.md) for grades · [009-hermes-comparison.md](009-hermes-comparison.md) for Hermes parity · **[010-delightful-tui-plan.md](010-delightful-tui-plan.md)** for next-phase polish.

## Related prior specs

- [specs/05-improve-ux-tui.md](../05-improve-ux-tui.md) — width-adaptive tool display, context gauge (partially implemented)
- [specs/improve_plan/04-error-guidance.md](../improve_plan/04-error-guidance.md) — tool error self-healing
- [specs/steering/](../steering/) — mission steering UX

## Cross-references

→ [001-data-flow-map.md](001-data-flow-map.md) · [005-honest-assessment.md](005-honest-assessment.md) · [008-cross-ref-index.md](008-cross-ref-index.md)
