# 008 — EdgeCrab Current State

| Component | Location |
|-----------|----------|
| `LLMProvider` (Mode B) | `edgequake-llm` via `edgecrab-tools::create_provider_for_model` |
| Model catalog + aliases | `edgecrab-core` `ProxyConfig.model_aliases` |
| **OpenAI-compat proxy** | `crates/edgecrab-proxy/` |
| Gateway agent API | `edgecrab-gateway/.../api_server.rs` — **not** the proxy |

## Implemented

### CLI (`edgecrab-cli/src/proxy_cmd/`)

| Command | Purpose |
|---------|---------|
| `edgecrab proxy` | Help + config overview (no subcommand) |
| `edgecrab proxy setup [grok\|nous\|xai] [--yes]` | Guided preset: config + token + client snippet |
| `edgecrab proxy enable <preset>` | Add built-in upstream to `config.yaml` |
| `edgecrab proxy doctor` | Preflight: token, upstream auth, OAuth presets |
| `edgecrab proxy client [--show-token]` | Print `OPENAI_API_BASE` / Aider snippet |
| `edgecrab proxy start [--host] [--port] [--allow-public] [--provider KEY]` | Run server |
| `edgecrab proxy status` | Config summary |
| `edgecrab proxy upstreams` (alias `providers`) | Forward upstream table |
| `edgecrab proxy token {set,show,rotate}` | Local client bearer (not provider OAuth) |

Recipes and client snippets: `edgecrab-proxy/src/guide.rs` (`RECIPE_NOUS`, `RECIPE_XAI`, `grok` → `xai`).  
`apply_recipe` sets `default_forward_upstream` so `edgecrab proxy start` can run Hermes-style after setup (single upstream or explicit default).

### TUI slash command

| Slash | Behavior |
|-------|----------|
| `/proxy` | In-TUI setup wizard (Grok/xAI, Nous) — **exception UI**; default activation path |
| `/proxy setup` | Same wizard |
| `/proxy status` | Report overlay |
| `/proxy doctor` | Preflight overlay |
| `/proxy client` | Client snippet overlay (token redacted) |
| `/proxy enable grok` | Enable preset inline (no TUI) |

Shared hub: `edgecrab-cli/src/proxy_hub.rs` (DRY with `proxy_cmd/` and `proxy_setup_tui.rs`).

### HTTP (`edgecrab-proxy/src/server.rs`)

| Route | Behavior |
|-------|----------|
| `GET /v1/models` | Local aliases, or forward when `default_forward_upstream` / `--provider` |
| `POST /v1/chat/completions` | Mode B bridge or Mode A forward (`forward:<key>` / `--provider`) |
| `POST /v1/embeddings` | Forward (Mode A) or 501 (Mode B) |
| `GET /health`, `GET /v1/health` | Service + upstream readiness |
| `/v1/*` fallback | Forward-only mode (`--provider`) |

### Mode A (Hermes-style)

- `backend/adapter.rs` — `UpstreamAdapter` trait + `StaticBearerAdapter`
- `backend/auth_store.rs` — `HermesAuthFileAdapter` (`adapter: hermes_auth`) reads `agent_key` from auth.json (no refresh)
- `backend/nous/` — `NousPortalAdapter` (`adapter: nous_portal`) OAuth refresh, invoke JWT, 401 retry, **inference URL allowlist**, **terminal OAuth quarantine**, `auth.json` file lock
- `backend/xai/` — `XaiGrokAdapter` (`adapter: xai_oauth`) OIDC refresh, credential pool rotation on 429, `/responses` path
- `backend/auth_file.rs` — shared auth.json load/save + credential pool helpers
- `backend/factory.rs` — `build_forward_adapter` (static | hermes_auth | nous_portal | xai_oauth)
- `backend/forwarder.rs` — verbatim pass-through, bearer swap, query string preserved
- `registry.rs` — upstream registry, preflight auth
- Config: `proxy.forward_upstreams`, `default_forward_upstream`, `auth_hint`

### Mode B (provider bridge)

- `wire/messages.rs`, `wire/sse.rs` — OpenAI ↔ `edgequake_llm`
- `backend/provider.rs` — `LLMProvider` + SSE from `StreamChunk`

### Security

- `~/.edgecrab/proxy-token` (0600), timing-safe Bearer
- Loopback default; `--allow-public` for non-local bind
- Optional CORS via `proxy.cors_allow_origins`

### Tests

- `cargo test -p edgecrab-proxy` — 72 tests (47 unit + 25 HTTP e2e) + `edgecrab-cli` proxy CLI/slash e2e (5)
- E2E harness: `enable_e2e_direct_http()` + `e2e_http_client()` (no system proxy / no flock on loopback mocks)

## Still Missing (024 / manual)

1. **Other OAuth adapters** (Claude Pro, ChatGPT Pro, Copilot) — [024-oauth-providers/](../024-oauth-providers/) — Nous + xAI Grok proxy adapters implemented
2. **Live SDK smoke** — OpenAI Python SDK / Aider against real Anthropic (manual; wire shape covered by e2e)
3. **edgequake-llm public `wire::openai`** — types remain in `edgecrab-proxy` until upstream exposes them

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
