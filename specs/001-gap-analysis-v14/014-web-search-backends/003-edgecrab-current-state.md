# 014 — EdgeCrab Current State (post Hermes gap-closure)

| Component | File |
|-----------|------|
| `web_search` tool | `crates/edgecrab-tools/src/tools/web/search/tool.rs` |
| `WebSearchBackend` trait | `crates/edgecrab-tools/src/tools/web/search/backend.rs` |
| Credential resolution (DRY) | `crates/edgecrab-tools/src/tools/web/search/backend_settings.rs` |
| Hermes response envelope | `crates/edgecrab-tools/src/tools/web/search/response.rs` |
| Exa / Parallel | `backends/exa.rs`, `backends/parallel.rs` |
| Fallback chain | `crates/edgecrab-tools/src/tools/web/search/chain.rs` |
| Rate limiter | `crates/edgecrab-tools/src/tools/web/search/rate_limit.rs` |
| Plugin registry | `crates/edgecrab-tools/src/tools/web/search/registry.rs` |
| Config (`web_search.*`) | `crates/edgecrab-core/src/config.rs` + `AppConfigRef.web_search` |
| SSRF guard | `edgecrab-security::url_safety` (used by all backends) |
| Website blocklist | `edgecrab-security::website_policy` (Hermes `security.website_blocklist`) |
| E2E proof | `specs/001-gap-analysis-v14/014-web-search-backends/e2e/` |

## Implemented

1. **`WebSearchBackend` trait** with normalized `SearchResult` (rank, title, url, snippet, source).
2. **Seven search backends** (SearXNG, Brave, DDGS, Firecrawl, Tavily, Exa, Parallel) behind cargo features.
3. **`BackendChain`** — fallback on RateLimit/Timeout/5xx/Network; skip unconfigured in multi-backend chains; **fail-fast** on explicit single-backend selection.
4. **Per-backend RPS token bucket** + post-429 cooldown.
5. **`backend_settings`** — single source for `api_key` / `endpoint` / `timeout_secs` (config.yaml → env fallback).
6. **Hermes-compatible `data.web[]`** in tool output alongside native `results[]`.
7. **`max_results` clamp 1–100** (Hermes parity).
8. **`register_web_search_backend()`** for plugin/runtime registration.
9. **Website blocklist** — `security.website_blocklist` gates `web_extract` (and shared URL validators).

## SOLID / DRY layout

| Principle | How |
|-----------|-----|
| **S** | Each backend is one file implementing `WebSearchBackend`; chain only orchestrates. |
| **O** | New backends via trait + `register_web_search_backend` without chain changes. |
| **L** | All backends return the same `SearchResult` shape. |
| **I** | Thin trait: `name`, `is_available`, `search`. |
| **D** | Chain depends on `Arc<dyn WebSearchBackend>`, not concrete APIs. |
| **DRY** | Credentials in `backend_settings`; HTTP in `http.rs`; policy in `website_policy`. |

## Remaining gaps vs Hermes

See [e2e/results.md](e2e/results.md) — xAI search, extract registry unification, blocklist on browser/vision, DDGS library parity.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md) · [002-hermes-reference.md](002-hermes-reference.md)
