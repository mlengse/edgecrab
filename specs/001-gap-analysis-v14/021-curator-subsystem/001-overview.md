# 021 — Curator Subsystem

**Tier:** C | **Impact:** 2 | **Value-per-Effort:** 3 | **Risk:** 2
**Primitive moved:** Memory (autonomous maintenance)

## Why It Matters (First Principles)

Memory is a write-mostly file. Over weeks, MEMORY.md fills with
duplicate entries, stale facts, and contradictory notes. The agent
reads it every turn, so noise becomes tax. Hermes v0.14 added a
background "curator" — a low-priority agent that periodically rewrites
MEMORY.md to dedupe, archive stale, and consolidate themes.

## The Gap

EdgeCrab never grooms memory. Drift accumulates linearly.

## What EdgeCrab Gets Wrong Today

After 30 sessions, MEMORY.md is 8 KB of redundant bullets the agent
re-reads every turn — wasted context, slow convergence, contradictory
"facts" leading to lower quality answers.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
