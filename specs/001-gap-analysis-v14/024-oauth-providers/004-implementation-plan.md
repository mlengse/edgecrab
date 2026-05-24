# 024 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/providers/oauth/ (NEW module)            │
   │                                                                  │
   │   mod.rs              OAuthProvider trait + registry             │
   │   pkce.rs             PKCE codes (S256 challenge/verifier)       │
   │   callback_server.rs  ephemeral 127.0.0.1 listener for code      │
   │   device_flow.rs      device-code polling (Copilot, fallback)   │
   │   token_store.rs      file-backed JSON token store (0600)        │
   │                                                                  │
   │   providers/                                                     │
   │     claude_pro.rs                                                │
   │     chatgpt_pro.rs                                               │
   │     super_grok.rs                                                │
   │     copilot.rs                                                   │
   │                                                                  │
   │   Each implements:                                               │
   │     async fn login(&self) -> TokenSet                            │
   │     async fn refresh(&self, t: &TokenSet) -> TokenSet            │
   │     fn chat_endpoint(&self) -> Url                               │
   │     fn build_headers(&self, t: &TokenSet) -> HeaderMap           │
   │     fn map_to_messages(&self, anthropic_or_openai...) -> Bytes   │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/model_router.rs                          │
   │                                                                  │
   │   when model id is `claude-pro/opus`, route to OAuthProvider     │
   │   (Claude); on 401 → refresh + retry once.                       │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-cli/src/commands.rs                                │
   │                                                                  │
   │   /login claude-pro | chatgpt-pro | super-grok | copilot         │
   │   /logout <provider>                                             │
   │   /providers (list authenticated providers)                      │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-core/src/providers/oauth/` |
| **Token store** | `~/.edgecrab/oauth/<provider>.json` chmod 0600 |
| **Callback server** | ephemeral random-port HTTP listener; close after first request; opens default browser to provider's auth URL |
| **Device flow** | for headless / SSH scenarios; print code + URL; poll until granted |
| **Refresh logic** | wrap each request; on 401 try refresh once; on refresh failure → emit "re-login required" error |
| **Slash commands** | `/login`, `/logout`, `/providers` in `crates/edgecrab-cli/src/commands.rs` |
| **Model IDs** | `claude-pro/sonnet-4.5`, `chatgpt-pro/gpt-5`, `super-grok/grok-4`, `copilot/gpt-5` — routed to OAuth providers via `model_router` |
| **Catalog** | extend `model_catalog_default.yaml` with `claude-pro` etc. provider blocks |
| **Rate-limit surfacing** | parse provider rate-limit headers and surface in `/usage` |
| **Tests** | mock OAuth server using `wiremock`; full round-trip integration tests per provider |

## Risks

- Provider TOS may forbid programmatic use of consumer flows. Document
  clearly that users are responsible for compliance with their chosen
  provider's TOS.
- Endpoints + headers change frequently. Adapters must be small,
  versioned, isolated.
- Browser-based OAuth must work behind firewalls — fallback to device
  flow.

## DRY / SOLID Notes

- **SRP:** PKCE / device-flow / callback / token-store / per-provider
  adapter are separate modules.
- **OCP:** add a new OAuth provider = one new file implementing
  `OAuthProvider`.
- **DIP:** model router depends on the trait; no concrete provider
  imports.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- This enables: [../008-openai-compat-proxy/](../008-openai-compat-proxy/)
