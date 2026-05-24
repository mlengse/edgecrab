# 014 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-tools/src/tools/web/                        │
   │                                                                  │
   │   search.rs        (ToolHandler — uses BackendChain)             │
   │   extract.rs       (existing extract logic — unchanged)          │
   │   crawl.rs         (existing recursive crawl — unchanged)        │
   │   backend.rs       (WebSearchBackend trait)                      │
   │   backends/                                                      │
   │     searxng.rs                                                   │
   │     brave.rs                                                     │
   │     ddgs.rs                                                      │
   │     google_cse.rs  (optional, for users with API key)            │
   │     mock.rs        (test fixture)                                │
   │   chain.rs         (BackendChain — fallback orchestrator)        │
   │   rate_limit.rs    (per-backend rps token bucket)                │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Refactor** | Split `crates/edgecrab-tools/src/tools/web.rs` into `web/` module |
| **Trait** | `crates/edgecrab-tools/src/tools/web/backend.rs` — `#[async_trait] pub trait WebSearchBackend { fn name(&self) -> &str; async fn search(q, opts) -> Result<Vec<SearchResult>, SearchError>; }` |
| **Backends** | one file per backend in `backends/` |
| **Chain** | `chain.rs` — `BackendChain { primary, fallbacks: Vec<...> }`; rotate on `RateLimit`/`Timeout`/`Server(5xx)` |
| **Rate limiter** | `rate_limit.rs` — `governor`-style token bucket per backend; chain skips a backend whose budget is exhausted |
| **Config** | `web_search.primary: "searxng"`, `web_search.fallbacks: ["brave", "ddgs"]`, per-backend sub-config |
| **Plugin hook** | `BackendRegistry::register(name, Arc<dyn WebSearchBackend>)` — called by plugins via `tool_override`-adjacent API |
| **Tests** | mock backend that fails in N modes; chain test that confirms each fallback path |

## Failure Modes & Fallback Rules

| Error | Action |
|-------|--------|
| `RateLimit` | mark backend cooldown N seconds, try next |
| `Timeout` (configurable, default 8s) | try next |
| `Server(5xx)` | try next; record failure |
| `Network` (DNS/connection refused) | try next |
| `BadRequest` (4xx other than 429) | do NOT fall back — surface error |
| `EmptyResults` | NOT a failure; return empty list |

If all backends fail → tool returns `SearchError` with chain summary
(which backends were tried, what they returned).

## Security

- All outbound URLs go through `edgecrab-security::ssrf::is_safe_url`.
- API keys read from env or `~/.edgecrab/config.yaml` `web_search.<backend>.api_key`; never logged.
- SearXNG URL must be http/https and pass SSRF check (so users can't be
  tricked into pointing at a private metadata endpoint).

## DRY / SOLID Notes

- **OCP:** new backend = new file. No changes to the tool or chain.
- **DIP:** chain depends on the trait; backends depend on `reqwest`
  (already a workspace dep).
- **SRP:** backend / chain / rate limiter / tool surface — four files.
- **DRY:** `SearchResult` is the single shared type; all backends
  normalise to it.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Plugin override demo case for: [../009-pluggable-providers-plugins/](../009-pluggable-providers-plugins/)
