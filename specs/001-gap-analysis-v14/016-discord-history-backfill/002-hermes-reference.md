# 016 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Discord adapter | `hermes-agent/integrations/discord/adapter.py` |
| Backfill | `discord.py` → `channel.history(limit=N)` (async iterator) |
| Trigger | on first activation in a channel, or on `/backfill` slash command |
| Seeding | messages converted to OpenAI-format `user` / `assistant` roles based on author (bot vs human) and injected into session history before the next live message |
| Cap | default 50 messages; configurable `discord.backfill_limit` |
| Cache | per-channel marker (`last_seen_message_id`) prevents re-backfilling on every restart |

## Conversion Rules

- Bot's own messages → role `assistant`.
- Human messages → role `user` (prefixed `[username]: `).
- Attachments → noted as `[attachment: name (mime)]` text; not downloaded.
- Embeds → text representation of title + description.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
