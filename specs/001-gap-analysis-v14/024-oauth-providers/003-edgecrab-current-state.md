# 024 — EdgeCrab Current State

| Area | Status | Location |
|------|--------|----------|
| Nous Portal device OAuth | **Done** | `crates/edgecrab-proxy/src/backend/nous/device_flow.rs` |
| Nous refresh + invoke JWT | **Done** | `backend/nous/refresh.rs`, `jwt.rs` |
| Auth store (`providers.nous`) | **Done** | `backend/auth_file.rs` — Hermes-compatible `auth.json` |
| CLI `edgecrab auth add nous` | **Done** | `crates/edgecrab-cli/src/auth_cmd.rs` |
| TUI `/auth add nous` | **Done** | same `auth_cmd` via slash dispatch |
| Proxy guide hint | **Done** | `RECIPE_NOUS.hermes_auth_cmd` → `edgecrab auth add nous` |

## Nous login flow (Hermes parity)

1. `POST {portal}/api/oauth/device/code` → user opens `verification_uri_complete`.
2. Poll `POST {portal}/api/oauth/token` with `urn:ietf:params:oauth:grant-type:device_code`.
3. `finalize_nous_state` — refresh if invoke JWT not yet usable; mint `agent_key`.
4. Persist under `providers.nous` in `~/.edgecrab/auth.json` (shared path with Hermes when migrated).

Commands:

```bash
edgecrab auth add nous          # device login (no --token)
edgecrab auth login nous        # alias
edgecrab auth status nous
edgecrab auth remove nous
```

## Still missing (full 024 scope)

1. PKCE browser flows for Claude Pro / ChatGPT Pro / SuperGrok.
2. Copilot device flow in Rust (Copilot today: GitHub PAT via `edgecrab auth add copilot --token`).
3. xAI OAuth login (`edgecrab auth add xai-oauth`) — forwarder reads Hermes-shaped state only.
4. Per-provider token dirs under `~/.edgecrab/oauth/` (optional; Nous uses unified `auth.json`).
5. Model-router OAuth providers (024 is proxy-forwarder scoped today).

## Tests

| Test | Crate |
|------|-------|
| `device_flow_round_trip_mock_portal` | `edgecrab-proxy` lib |
| `resolves_nous_auth_target` | `edgecrab-cli` binary unit tests |
| Nous forwarder e2e (quarantine, 401 retry) | `edgecrab-proxy` `tests/e2e_*` |

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Proxy: [../008-openai-compat-proxy/003-edgecrab-current-state.md](../008-openai-compat-proxy/003-edgecrab-current-state.md)
