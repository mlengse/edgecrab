# 027 — `x_search` Tool

**Tier:** C | **Impact:** 2 | **Value-per-Effort:** 3 | **Risk:** 2
**Primitive moved:** Capability (real-time social search)

## Why It Matters (First Principles)

Web search returns indexed-and-stale pages. Real-time queries — "what
is happening with X stock right now", "is the API down" — need a
realtime social signal. Hermes v0.14 ships `x_search` over the X/
Twitter API.

## The Gap

EdgeCrab has no X search tool.

## What EdgeCrab Gets Wrong Today

For real-time incident triage, breaking news, or sentiment queries the
agent must scrape news sites that lag by minutes-to-hours.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
