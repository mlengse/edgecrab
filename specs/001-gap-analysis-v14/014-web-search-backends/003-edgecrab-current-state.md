# 014 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `web_search` tool | `crates/edgecrab-tools/src/tools/web.rs` |
| SSRF guard | `crates/edgecrab-security/src/ssrf.rs` |

## What Is Missing

1. No `WebSearchBackend` trait.
2. No SearXNG, Brave, or DDGS impls (only the current one).
3. No fallback chain.
4. No per-backend rate limit accounting.
5. No way for a plugin (feature 009) to register a new backend.

## Honest Assessment

The smallest Tier B feature and a clean test case for the plugin
override mechanism in feature 009. Should ship close to 009 to
demonstrate end-to-end extensibility.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
