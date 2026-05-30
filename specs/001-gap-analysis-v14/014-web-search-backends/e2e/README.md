# 014 — Web Search Backends E2E Proof

Run the full suite:

```bash
make test-web-search-e2e   # unit + mock integration + Docker SearXNG live
```

Or step by step:

```bash
# Unit tests (30+ cases — chain, parsers, config, SSRF, rate limit)
cargo test -p edgecrab-tools --lib web::search -- --nocapture

# Mock edge cases (Hermes parity — no network)
cargo test -p edgecrab-tools --test web_search_e2e_edge -- --test-threads=1 --nocapture

# Core integration + live backends
cargo test -p edgecrab-tools --test web_search_e2e -- --test-threads=1 --nocapture
cargo test -p edgecrab-tools --test web_search_e2e -- --include-ignored --test-threads=1 --nocapture

# Agent + Copilot gpt-5-mini
cargo test -p edgecrab-core --test web_search_e2e -- --include-ignored --nocapture
```

## Docker SearXNG (real backend, no public instance needed)

Local SearXNG runs on `http://127.0.0.1:8888`. SSRF normally blocks loopback; E2E sets `EDGECRAB_E2E_SSRF_ALLOW_LOCALHOST=1` (loopback only — not LAN/private ranges).

**One command** (starts Docker, waits for JSON API, runs test, tears down):

```bash
specs/001-gap-analysis-v14/014-web-search-backends/e2e/run-searxng-e2e.sh
```

Manual flow:

```bash
cd specs/001-gap-analysis-v14/014-web-search-backends/e2e
docker compose -f docker-compose.searxng.yml up -d
curl 'http://127.0.0.1:8888/search?q=test&format=json' | head

export SEARXNG_URL=http://127.0.0.1:8888
export EDGECRAB_E2E_SSRF_ALLOW_LOCALHOST=1
export EDGECRAB_SEARXNG_DOCKER_E2E=1
cargo test -p edgecrab-tools --test web_search_e2e e2e_searxng_docker_live_search \
  -- --include-ignored --test-threads=1 --nocapture

docker compose -f docker-compose.searxng.yml down
```

Keep the container running for debugging: `SEARXNG_E2E_LEAVE_RUNNING=1 ./run-searxng-e2e.sh`

## Hermes parity matrix

| Hermes scenario | EdgeCrab test |
|-----------------|---------------|
| Empty results = success | `e2e_empty_results_success_not_fallback`, `parse_empty_results_is_success_shape` |
| 429 → fallback | `e2e_fallback_from_rate_limited_primary`, `chain_fallback_policy_end_to_end` |
| 503/timeout/network → fallback | `e2e_503_fallback_through_tool`, `e2e_timeout_fallback_through_tool`, `e2e_network_error_fallback` |
| 400/403 → no fallback | `e2e_400_no_fallback_through_tool`, `e2e_403_no_fallback_through_tool` |
| Explicit backend override (no chain) | `e2e_explicit_backend_override_skips_chain`, `explicit_override_single_backend_no_chain` |
| `backend: auto` uses config chain | `auto_override_uses_full_chain` |
| Env `EDGECRAB_WEB_SEARCH_BACKEND` | `e2e_env_backend_override`, `env_backend_override_wins_over_config` |
| Unconfigured backend skipped | `e2e_skips_unconfigured_searxng_falls_back_to_ddgs` |
| `brave-free` alias | `e2e_brave_free_alias_resolves`, `backend_alias_brave_free_maps_to_brave` |
| `duckduckgo`/`ddg` alias | `e2e_duckduckgo_alias_resolves_to_ddgs`, `backend_alias_duckduckgo_maps_to_ddgs` |
| Plugin register + overwrite | `e2e_plugin_registered_backend_via_tool`, `e2e_plugin_overwrites_same_name` |
| Unknown backend error | `e2e_unknown_backend_override_errors`, `unknown_backend_in_chain_returns_error` |
| Result shape `{rank,title,url,snippet,source}` | `e2e_result_shape_parity`, SearXNG/Brave parse tests |
| SearXNG score sort + limit | `parse_normalizes_score_sorted_results`, `parse_respects_limit` |
| Brave-free JSON normalization | `parse_normalizes_brave_free_shape` |
| max_results clamp 1–20 | `max_results_clamped_to_twenty` |
| SSRF on SearXNG URL | `ssrf_blocks_private_searxng_url`; Docker e2e uses `EDGECRAB_E2E_SSRF_ALLOW_LOCALHOST=1` (loopback only) |
| API key redaction | `api_key_redacted_from_error_text` |
| All-fail summary | `e2e_all_fail_returns_descriptive_error` |
| Live DDGS (no key) | `e2e_ddgs_search_without_api_key` — no `bing.com/ck/a` in URLs (skips on bot-challenge) |
| **DDGS edge cases (mock)** | `web_search_ddgs_edge_e2e.rs` — Bing decode, turnstile false-positive, HTTP 202, lite/html parsers |
| **Virtual tmp file tools** | `file_write_tmp_e2e.rs` — `/tmp/…` and `tmp/files/…` write+read roundtrip |
| Live SearXNG | `e2e_searxng_search_when_configured` (needs `SEARXNG_URL`) |
| **Docker SearXNG (real JSON API)** | `e2e_searxng_docker_live_search` via `run-searxng-e2e.sh` |
| Live Brave | `e2e_brave_search_when_key_set` (needs `BRAVE_API_KEY`) |
| Live Tavily / Firecrawl | `e2e_tavily_search_when_key_set`, `e2e_firecrawl_search_when_key_set` |
| Agent invokes tool | `e2e_agent_web_search_with_copilot_gpt5_mini`, `e2e_agent_web_search_fallback_chain_ddgs` |

Results are captured in [`results.md`](results.md) after each run.
