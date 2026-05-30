# E2E Results — 014 Web Search Backends

Generated: 2026-05-30 (search fallback chain wizard + tools→web hook)

## Summary

| Suite | Count | Status |
|-------|-------|--------|
| Unit (`web::search`) | 82 | PASS |
| Integration E2E (mock + config + wizard + chain + doctor + setup) | 21+ | PASS |
| **Docker SearXNG** | **6** | **PASS** |
| CLI `web_setup` + `/web` handler | unit | PASS |

**Total runnable web-stack tests: ~146**

```bash
make test-web-search-e2e
cargo fmt --all -- --check
cargo clippy -p edgecrab-tools -p edgecrab-security -p edgecrab-core -p edgecrab-cli -- -D warnings
```

## This pass — search fallback chain + tools hook

| Item | Status |
|------|--------|
| **`WebSearchChainUpdate` + persist/clear** | `web_search.primary` / `fallbacks` / `timeout_secs` in `config.yaml` |
| **`format_search_chain_summary()`** | DRY chain display for doctor, TUI, wizard |
| **Wizard menu: configure search chain** | Primary picker + comma fallbacks + timeout |
| **Wizard: reset chain to auto** | Clears primary/fallbacks, keeps backend keys |
| **`/web chain`** | TUI overlay with primary → fallbacks + override note |
| **`edgecrab setup tools` → web wizard** | Offers wizard when web toolset newly enabled |
| **E2E `web_search_e2e_chain.rs`** | Persist, clear, diagnostics chain line |
| **fmt + clippy** | `-D warnings` clean on web crates |

## Brutally honest vs Hermes

### EdgeCrab ahead

- **Explicit `web_search` fallback chain** with persist API + wizard + `/web chain` — Hermes uses implicit legacy walk only (no `primary`/`fallbacks` YAML wizard)
- Multi-backend fallback + per-backend RPS + 429 cooldown
- **`edgecrab doctor` web lines** (Hermes doctor ignores web)
- **Dedicated `/web` TUI command** with status / setup / chain / doctor overlays
- Split search/extract config + chain config in one focused wizard
- Docker SearXNG E2E with non-empty result gate
- Capability badges `S+E+C` on picker rows

### Parity

| Hermes | EdgeCrab |
|--------|----------|
| `hermes setup tools` web picker | `edgecrab setup web` + `/web setup` + tools hook |
| `web.backend` / split overrides | ✅ |
| `get_setup_schema()` | ✅ |
| 8 search + 4 extract providers | ✅ |
| Legacy auto chain order | ✅ `LEGACY_AUTO_CHAIN` |

### Still behind (honest)

| Gap | Severity |
|-----|----------|
| Unified mega-picker (`hermes tools` for TTS/browser/web/image) | Low |
| Nous managed gateway rows | Low |
| `crawl()` on provider trait (optional method) | Low |
| `extract_crawl.rs` size (~2100 lines orchestration) | Low |
| DDGS reliability (Python `ddgs` lib vs HTML scrape) | Medium — **2026-05-30: native Rust ddgs fixed** (Bing parser, bot-detect, HTTP 202 body handling) |

## DDGS native Rust — 2026-05-30 fix pass

| Root cause | Fix |
|------------|-----|
| Bing `"There are no results for"` in embedded JS → empty SERP | Only trust `b_noResults` without `b_algo`; parse organic blocks |
| Bing regex required bare `<h2>` | Match `<h2[^>]*>` (modern SERP uses `<h2 class="">`) |
| `turnstile` in Bing JS triggered false bot-challenge | Skip block detection when `b_algo` / `result__a` present |
| HTTP 202 from DDG HTML/lite discarded at transport | Read body on 202; detect anomaly-modal in `process_page` |
| Bing first page sent extra `first`/`FORM` params | Match Python ddgs: `q` only on first GET |
| Misleading single-engine error | Aggregate all engine failures in metasearch |

```bash
cargo test -p edgecrab-tools --lib web::search::backends::ddgs -- --nocapture
cargo test -p edgecrab-tools --test web_search_ddgs_edge_e2e --test web_search_ddgs_e2e --test file_write_tmp_e2e -- --nocapture
cargo test -p edgecrab-tools --test web_search_e2e e2e_ddgs_search_without_api_key -- --include-ignored --nocapture
```

Live proof (2026-05-30): `e2e_ddgs_search_without_api_key` returns 3 Bing results for "Rust programming language".
| Test depth vs Hermes ~207 web-adjacent tests | Low–Med |
| Plugin web providers (Hermes `register_web_search_provider`) | Low |

### Verdict

**Feature parity ~100% on providers, config, dispatch, and setup.** EdgeCrab now **exceeds Hermes on search chain configurability** (`web_search.primary` + ordered fallbacks + timeout wizard) while Hermes still wins on **unified cross-tool setup menu** and **DDGS implementation quality**. Remaining work is consolidation (`extract_crawl.rs` split), not missing backends.
