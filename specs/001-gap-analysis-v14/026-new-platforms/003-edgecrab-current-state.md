# 026 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Adapter trait | `crates/edgecrab-gateway/src/platforms/mod.rs` (`PlatformAdapter`) |
| 17 adapters | telegram, discord, slack, whatsapp, signal, webhook, sms, matrix, mattermost, dingtalk, homeassistant, api_server, email, feishu, wecom, bluebubbles, weixin |
| Delivery router | `crates/edgecrab-gateway/src/run.rs` |
| Session manager | `crates/edgecrab-gateway/src/session.rs` |

## What Is Missing

1. LINE adapter.
2. SimpleX adapter.
3. Google Chat adapter.
4. MS Teams adapter (Bot Framework).

## Honest Assessment

The trait already exists; existing adapters are templates. Each new
platform is mechanical — webhook signature verification + REST client
+ message-mapping. SimpleX requires a local bridge subprocess (CLI),
adding subprocess management.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
