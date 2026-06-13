# 014 — Acceptance Criteria

## Functional

- [x] `web_search` with `primary: "searxng"` succeeds against a public
      SearXNG instance (`e2e_searxng_search_when_configured`) **and** local Docker
      (`e2e_searxng_docker_live_search` via `e2e/run-searxng-e2e.sh` — verified PASS).
- [x] Setting Brave key and `primary: "brave"` works (`e2e_brave_search_when_key_set`).
- [x] DDGS works with no key (`e2e_ddgs_search_without_api_key` — live proof in `e2e/results.md`).
- [x] Fallback: configure primary that always 429s; chain falls back to
      next backend and returns results (`e2e_fallback_from_rate_limited_primary`).
- [x] All-fail: returns descriptive `SearchError` listing tried backends.
- [x] Config `web_search.backends.*.api_key` / `endpoint` used at runtime (not env-only).
- [x] Hermes `data.web[]` shape present in tool output.
- [x] `max_results` capped at 100 (Hermes parity).
- [x] Explicit backend fail-fast when misconfigured (`e2e_brave_free_alias_resolves`, unit `explicit_unconfigured_backend_fails_fast`).
- [x] Config-only SearXNG endpoint (`e2e_searxng_docker_config_endpoint_without_env`).
- [x] Plugin can register a new backend at runtime (`register_web_search_backend` + `e2e_plugin_overwrites_same_name`).

## Security

- [x] SearXNG endpoint URL validated via SSRF guard (`ssrf_blocks_private_searxng_url`).
- [x] API keys never appear in logs (`api_key_redacted_from_error_text`).

## Code Quality

- [x] `cargo clippy -p edgecrab-tools` clean for new `web/search` module (pre-existing `computer_use` warnings remain elsewhere in crate).
- [x] Website blocklist gates `web_extract` (`web_extract_policy_e2e`, `edgecrab-security/website_policy`).
- [x] ≥ 12 tests across backends + chain (65 unit + 33 mock integration + 6 Docker + 2 config + 2–3 agent).
- [x] Backends gated behind cargo features (`searxng`, `brave`, `ddgs`, `firecrawl`, `tavily`).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
- [e2e/results.md](e2e/results.md)
