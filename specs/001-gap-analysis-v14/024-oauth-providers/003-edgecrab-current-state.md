# 024 — EdgeCrab Current State

| Area | Status | Location |
|------|--------|----------|
| Nous Portal device OAuth | **Done** | `edgecrab-proxy/backend/nous/device_flow.rs` |
| **xAI Grok PKCE OAuth** | **Done** | `edgecrab-proxy/oauth/` + `backend/xai/oauth_login.rs` |
| xAI refresh + forwarder | **Done** | `backend/xai/refresh.rs`, `adapter.rs` |
| Auth store (`providers.*`) | **Done** | `backend/auth_file.rs` — Hermes-compatible `auth.json` |
| CLI `edgecrab auth add grok` | **Done** | `edgecrab-cli/src/auth_cmd.rs` |
| **Claude Pro OAuth** | **Done** | `edgecrab-core/src/oauth/anthropic.rs` → `.anthropic_oauth.json` |
| **ChatGPT Pro / Codex OAuth** | **Done** | `edgecrab-core/src/oauth/codex.rs` → `auth.json` |
| OAuth env injection for `anthropic/…` | **Done** | `edgecrab-core/src/oauth/runtime.rs` + `create_provider_async` |
| Copilot device login | **Done** (separate path) | `edgequake_llm` GitHub device flow via `auth_cmd` |
| Proxy recipes | **Done** | `guide.rs` — Nous + Grok |

## Grok / xAI login (Hermes parity)

1. OIDC discovery `https://auth.x.ai/.well-known/openid-configuration`
2. PKCE loopback on `http://127.0.0.1:56121/callback` (port fallback)
3. Token exchange echoes `code_challenge` + `code_verifier` (xAI requirement)
4. Persist `providers.xai-oauth` with `tokens`, `discovery`, `auth_mode: oauth_pkce`

```bash
edgecrab auth add grok          # primary alias (SuperGrok / X Premium+)
edgecrab auth add xai-oauth     # canonical provider id
edgecrab auth status grok
edgecrab auth remove grok
edgecrab proxy enable grok && edgecrab proxy start --provider xai
```

Remote / SSH:

```bash
EDGECRAB_AUTH_NO_BROWSER=1 edgecrab auth add grok
EDGECRAB_AUTH_MANUAL_PASTE=1 edgecrab auth add grok
```

## Still missing (full 024 scope)

1. Dedicated `OAuthProvider` trait module per 004 plan (today: focused modules in `edgecrab-core/src/oauth/`).
2. HTTP mock round-trip e2e for Claude/Codex token exchange (unit tests for auth store + PKCE only).
3. Native 401 refresh-and-retry inside edgequake-llm providers (EdgeCrab adds a CLI provider wrapper for OAuth retries).
4. edgequake-llm first-class `openai-codex` variant (EdgeCrab uses `openai-compatible` + env bridge).

## Tests

| Test | Crate |
|------|-------|
| `device_flow_round_trip_mock_portal` | `edgecrab-proxy` (Nous) |
| `token_exchange_round_trip_mock_auth`, `full_login_with_simulated_callback` | `edgecrab-proxy` (xAI PKCE) |
| `resolves_grok_auth_target` | `edgecrab-cli` |
| Grok/xAI forwarder e2e | `edgecrab-proxy/tests/e2e_*` |

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Proxy: [../008-openai-compat-proxy/](../008-openai-compat-proxy/)
