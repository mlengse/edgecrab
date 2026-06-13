# 024 — Acceptance Criteria

## Nous Portal (implemented subset)

- [x] `edgecrab auth add nous` runs device-code OAuth against Nous Portal.
- [x] Tokens stored in `~/.edgecrab/auth.json` under `providers.nous` (Hermes-shaped).
- [x] `edgecrab auth status nous` / `remove nous` / TUI `/auth` equivalents.
- [x] Mock portal test: `device_flow_round_trip_mock_portal`.
- [x] Proxy `NousPortal` adapter refresh-on-401 + invoke JWT (008 e2e).

## xAI Grok / SuperGrok (implemented subset)

- [x] `edgecrab auth add grok` / `xai-oauth` runs PKCE loopback OAuth (Hermes `auth.py` parity).
- [x] Tokens stored under `providers.xai-oauth` with `discovery` + `oauth_pkce` mode.
- [x] Token exchange sends `code_verifier` and echoes `code_challenge` (xAI #26990).
- [x] `edgecrab auth status grok` / `remove grok` / TUI `/auth` equivalents.
- [x] Mock tests: `token_exchange_round_trip_mock_auth`, loopback callback test.
- [x] Proxy `XaiOauth` adapter refresh-on-401 (008 e2e).

## Claude Pro / ChatGPT Pro (implemented subset)

- [x] `edgecrab auth add claude-pro` — PKCE + paste code → `~/.edgecrab/.anthropic_oauth.json` (0600).
- [x] `edgecrab auth add chatgpt-pro` — device code → `auth.json` `providers.openai-codex`.
- [x] `/login claude-pro` / `/login chatgpt-pro` — TUI terminal handoff (Copilot-style).
- [x] `/providers` — alias for `/auth list` (subscription + API-key status).
- [x] Model catalog `claude-pro` + `chatgpt-pro` blocks; `chatgpt-pro/gpt-5.4` via `openai-compatible`.
- [x] `anthropic/…` uses OAuth when `ANTHROPIC_API_KEY` unset (`inject_subscription_oauth_env`).
- [x] Unit tests: auth_store round-trip, codex credential probe, PKCE, auth target resolution.

## Full 024 — Functional (not done)

- [x] On 401, refresh token used; request retried; success (CLI OAuth wrapper layer).
- [x] On refresh failure, clear error: "re-login required: /login claude-pro".
- [ ] Mock HTTP full OAuth round-trip per provider (wiremock).
- [ ] Per-provider `~/.edgecrab/oauth/<id>.json` layout (optional; Hermes paths used instead).

## Security

- [ ] Token files are 0600 (verified by stat).
- [ ] No tokens logged at any tracing level.
- [ ] Callback server binds 127.0.0.1 only and shuts down after one
      request.
- [ ] PKCE verifier is at least 43 chars, base64url, random.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Mock tests for each provider's full OAuth round trip.

## Documentation

- [ ] `AGENTS.md` adds OAuth providers section with TOS reminder.
- [ ] `/help login` documents the flow per provider.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
