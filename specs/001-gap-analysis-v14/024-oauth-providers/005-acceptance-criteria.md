# 024 — Acceptance Criteria

## Nous Portal (implemented subset)

- [x] `edgecrab auth add nous` runs device-code OAuth against Nous Portal.
- [x] Tokens stored in `~/.edgecrab/auth.json` under `providers.nous` (Hermes-shaped).
- [x] `edgecrab auth status nous` / `remove nous` / TUI `/auth` equivalents.
- [x] Mock portal test: `device_flow_round_trip_mock_portal`.
- [x] Proxy `NousPortal` adapter refresh-on-401 + invoke JWT (008 e2e).

## Full 024 — Functional (not done)

- [ ] `/login claude-pro` opens browser → PKCE round trip → token
      stored in `~/.edgecrab/oauth/claude-pro.json` (chmod 0600).
- [ ] After login, `--model claude-pro/sonnet-4.5` routes to OAuth
      provider (verified via header inspection in mock test).
- [ ] On 401, refresh token used; request retried; success.
- [ ] On refresh failure, clear error: "re-login required: /login
      claude-pro".
- [ ] Same flow works for chatgpt-pro, super-grok, copilot.
- [ ] Copilot uses device-code flow (no callback server needed).
- [ ] `/providers` lists all currently authenticated providers + token
      expiry.
- [ ] `/logout chatgpt-pro` deletes token file.

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
