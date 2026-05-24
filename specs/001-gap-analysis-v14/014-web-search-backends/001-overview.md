# 014 — Pluggable Web-Search Backends (SearXNG + Brave + DDGS)

**Tier:** B | **Impact:** 4 | **Value-per-Effort:** 4 | **Risk:** 1
**Primitive moved:** Cost (often 0) + Reliability

## Why It Matters (First Principles)

`web_search` is one of the highest-frequency tools. Today EdgeCrab
depends on a single backend; cost, rate limits, and reliability all
flow from that choice. Hermes v0.14 ships a pluggable backend trait
with three concrete implementations:

1. **SearXNG** — self-hostable meta-search; **zero marginal cost** at
   the API layer. Best for privacy + cost-sensitive users.
2. **Brave Search API** — paid, high-quality results, generous free tier.
3. **DDGS (DuckDuckGo)** — free, no API key, modest rate limits, good
   fallback.

The right answer for a serious user is *all three with automatic
fallback* — primary fails or rate-limits → secondary fires.

## The Gap

EdgeCrab's `web_search` is monolithic. Switching backend requires code
change. No fallback. No way for a plugin to add a new backend.

## What EdgeCrab Gets Wrong Today

Heavy `web_search` use will hit rate limits silently or rack up API
costs on the single hardcoded provider, with no graceful degradation.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Uses plugin override pattern from: [../009-pluggable-providers-plugins/](../009-pluggable-providers-plugins/)
