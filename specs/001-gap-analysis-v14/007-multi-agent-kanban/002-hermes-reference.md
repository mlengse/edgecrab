# 007 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Card storage | SQLite-backed kanban table (per workspace) |
| Worker lease | Heartbeat + lease TTL; zombie reclaim if heartbeat stale |
| Tool surface | `kanban_card_create`, `kanban_card_claim`, `kanban_card_release`, `kanban_card_complete`, `kanban_card_list` |
| Supervisor pattern | A "lead" agent enqueues cards; N workers (spawned subagents) lease and execute |
| Dependencies | `depends_on: [card_id]` — DOING blocked until deps DONE |

## Card State Machine

```
   ┌──────┐  claim   ┌─────────┐  complete  ┌──────┐
   │ TODO │ ───────► │  DOING  │ ─────────► │ DONE │
   └──────┘          └────┬────┘            └──────┘
       ▲                  │
       │  zombie reclaim  │
       └──────────────────┘
       (heartbeat stale)
```

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
