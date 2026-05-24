# 026 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-gateway/src/platforms/                            │
   │                                                                  │
   │   line.rs        — webhook HTTP listener; signature verify;     │
   │                    Push API client; rich button mapping          │
   │   simplex.rs     — spawn `simplex-chat` CLI; WS bridge; map     │
   │                    chat IDs to opaque user keys                  │
   │   gchat.rs       — webhook or Pub/Sub; OAuth service account;   │
   │                    Card V2 for rich UI                           │
   │   teams.rs       — Bot Framework REST; Adaptive Cards;          │
   │                    conversation reference store                  │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │   Each impl provides:                                            │
   │       fn name() -> &'static str                                  │
   │       async fn run(rx, tx) → drive event loop                    │
   │       async fn send(target, msg)                                 │
   │       (optionally) async fn send_clarify(... buttons)            │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **LINE** | `crates/edgecrab-gateway/src/platforms/line.rs` — `axum` route at `/webhook/line`, HMAC verify, push via Messaging API |
| **SimpleX** | `crates/edgecrab-gateway/src/platforms/simplex.rs` — spawn `simplex-chat -p PORT`, connect WS, JSON-RPC commands |
| **Google Chat** | `crates/edgecrab-gateway/src/platforms/gchat.rs` — webhook + Card V2 builder helpers |
| **MS Teams** | `crates/edgecrab-gateway/src/platforms/teams.rs` — Bot Framework activity protocol; conversation refs stored in state DB |
| **Conversation ref store** | `gateway_conversations` SQLite table (Teams + Google Chat both need it for push) |
| **Env vars** | per platform; documented in `AGENTS.md` gateway table |
| **Signature middleware** | shared helper `verify_hmac(header, body, secret)` reused across LINE, Slack, Twilio |
| **Cargo features** | each platform behind a feature flag (`platform-line`, `platform-teams`, etc.) so bundle stays small for users who only need a subset |
| **Tests** | mock HTTP servers (`wiremock`) for each platform's send + receive |

## Risks

- Bot Framework JWT validation has many edge cases; use Microsoft's
  validation algorithm exactly.
- SimpleX upstream protocol churns; pin a CLI version and document
  compatibility.
- Google Chat OAuth service account distribution requires a Google
  Cloud project setup; document clearly.

## DRY / SOLID Notes

- **DRY:** HMAC verifier, JSON-RPC client, conversation-ref store all
  shared across adapters.
- **OCP:** new platform = new file implementing `PlatformAdapter`;
  zero changes elsewhere.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Clarify buttons rely on: [../015-native-clarify-buttons/](../015-native-clarify-buttons/)
