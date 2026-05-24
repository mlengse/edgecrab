# 015 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `clarify` tool | `crates/edgecrab-tools/src/tools/clarify.rs` |
| Telegram adapter | `crates/edgecrab-gateway/src/platforms/telegram.rs` |
| Discord adapter | `crates/edgecrab-gateway/src/platforms/discord.rs` |
| Slack adapter | `crates/edgecrab-gateway/src/platforms/slack.rs` |
| `DeliveryRouter` | `crates/edgecrab-gateway/src/run.rs` |
| `MEDIA://` protocol | intercept pattern in `DeliveryRouter` (reference for similar interception) |

## What Is Missing

1. `clarify` tool result has no structured `options` field — just text.
2. No `CLARIFY://` (or similar) sentinel protocol that
   `DeliveryRouter` could intercept.
3. No per-platform renderers for inline keyboards.
4. No callback-ingest plumbing (Telegram callback queries, Discord
   interaction endpoints) to receive the tap as input.

## Honest Assessment

Discord and Slack interaction handlers are the harder part — Telegram
inline keyboards are essentially "extra JSON field." We can ship
Telegram first (high ROI for the bot use case), then Discord, then
Slack.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
