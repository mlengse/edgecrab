# 016 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-gateway/src/platforms/discord.rs                  │
   │                                                                  │
   │   on_channel_first_seen(channel_id) {                            │
   │       if marker_exists(channel_id) return;                       │
   │       msgs = discord_api.get_messages(channel_id, limit=N);      │
   │       seed = convert_to_session_messages(msgs);                  │
   │       session_mgr.seed(channel_id, seed);                        │
   │       marker_save(channel_id, msgs[0].id);                       │
   │   }                                                              │
   │                                                                  │
   │   slash_handler("/backfill", channel_id, n) {                    │
   │       msgs = discord_api.get_messages(channel_id, limit=n);      │
   │       session_mgr.prepend(channel_id, convert(msgs));            │
   │   }                                                              │
   └──────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-state — channel_markers table (NEW)               │
   │                                                                  │
   │   CREATE TABLE channel_markers (                                 │
   │     platform TEXT NOT NULL,                                      │
   │     channel_id TEXT NOT NULL,                                    │
   │     last_seen_message_id TEXT NOT NULL,                          │
   │     backfilled_at INTEGER NOT NULL,                              │
   │     PRIMARY KEY (platform, channel_id)                           │
   │   );                                                             │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Modify** | `crates/edgecrab-gateway/src/platforms/discord.rs` — backfill on first-seen + `/backfill` slash command |
| **New helper** | `crates/edgecrab-gateway/src/backfill.rs` — conversion functions (bot vs human → role) reusable by future platforms |
| **Migration** | `crates/edgecrab-state/migrations/NNN_channel_markers.sql` |
| **Config** | `gateway.discord.backfill_limit: 50`, `gateway.discord.backfill_on_join: true` |
| **Tests** | mock Discord HTTP; assert messages converted with correct roles + author prefix |

## Compression Interaction

Backfilled history immediately consumes tokens. The compressor
(existing) treats them like any other history. To avoid blowing the
budget on a long channel, the backfill flow applies a *prune-on-seed*
step: if backfilled tokens > `gateway.discord.backfill_max_tokens`
(default 8 K), summarise old half before seeding.

## DRY / SOLID Notes

- **DRY:** the `backfill.rs` conversion helpers are generic over a
  `BackfillMessage` trait → can be reused by Telegram, Slack, Matrix
  when those add backfill.
- **SRP:** conversion in `backfill.rs`; storage in migration;
  orchestration in the discord adapter.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
