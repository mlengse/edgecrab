# 016 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Discord adapter | `crates/edgecrab-gateway/src/platforms/discord.rs` |
| Session manager | `crates/edgecrab-gateway/src/session.rs` |
| Channel directory | `crates/edgecrab-gateway/src/channel_directory.rs` |

## What Is Missing

1. No history-fetch call.
2. No conversion logic.
3. No seeding into session.
4. No `last_seen_message_id` marker persistence.
5. No `/backfill` slash command.

## Honest Assessment

Trivial feature, real impact. Discord API `GET /channels/{id}/messages?limit=N`
is a single call. Channel-marker storage piggybacks on the existing
session store. Done in a sitting.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
