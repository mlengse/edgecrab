# 014 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Backend abstraction | `WebSearchBackend` ABC in `hermes-agent/tools/web_search/backend.py` |
| SearXNG impl | `hermes-agent/tools/web_search/searxng.py` (HTTP JSON endpoint) |
| Brave impl | `hermes-agent/tools/web_search/brave.py` (api.search.brave.com) |
| DDGS impl | `hermes-agent/tools/web_search/ddgs.py` (HTML scrape / DDG public endpoints) |
| Fallback policy | `BackendChain` — primary; on `RateLimit`/`Timeout`/`5xx` try next |
| Config | per-backend section: api_key, endpoint, timeout, rps cap |

## Result Shape (normalised)

```
SearchResult {
  rank: int,
  title: str,
  url: str,
  snippet: str,
  source: str  # which backend produced it
}
```

All backends normalise into this shape so the tool's downstream consumer
is backend-agnostic.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
