# 026 — Hermes Reference

| Platform | Hermes file | Transport | Auth |
|----------|-------------|-----------|------|
| LINE | `hermes-agent/gateway/platforms/line.py` | Messaging API webhook + push | `LINE_CHANNEL_SECRET`, `LINE_CHANNEL_ACCESS_TOKEN` |
| SimpleX | `hermes-agent/gateway/platforms/simplex.py` | SimpleX CLI WebSocket bridge | local socket; no central server |
| Google Chat | `hermes-agent/gateway/platforms/gchat.py` | Google Chat REST API + Pub/Sub or webhook | OAuth service account |
| MS Teams | `hermes-agent/gateway/platforms/teams.py` | Bot Framework REST + Service URL | `TEAMS_APP_ID`, `TEAMS_APP_PASSWORD` |

## Notes per Platform

- **LINE**: signature header `X-Line-Signature` HMAC-SHA256, rich
  media + buttons supported.
- **SimpleX**: end-to-end encrypted; no platform metadata; identity
  per chat is opaque.
- **Google Chat**: card V2 components, slash commands native.
- **MS Teams**: Bot Framework JWT validation on inbound; Adaptive
  Cards for rich UI; conversation references must be stored to push.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
