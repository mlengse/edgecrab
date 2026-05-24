# 029 — Pareto Code Router

**Tier:** C | **Impact:** 2 | **Value-per-Effort:** 3 | **Risk:** 2
**Primitive moved:** Cost (intelligent dispatch)

## Why It Matters (First Principles)

A user asks "what time is it?" — routed to Opus is 100× more expensive
than necessary. A user asks "rewrite this 2KB Rust module" — routed
to Haiku produces noticeably worse code. Hermes v0.14 added a
heuristic "Pareto router": classify the request, route to the
cost-appropriate model.

## The Gap

EdgeCrab always uses the configured model regardless of request
complexity. Smart-routing logic is absent.

## What EdgeCrab Gets Wrong Today

Every turn pays peak model cost. A power user with 200 turns/day on
Opus could halve cost with no quality regression via routing.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
