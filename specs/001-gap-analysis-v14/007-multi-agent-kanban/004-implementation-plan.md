# 007 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────┐
   │              edgecrab-core/kanban                            │
   │                                                              │
   │   ┌──────────────────┐   ┌─────────────────────────────┐     │
   │   │ KanbanBoard      │──►│ SqliteBoard (default impl)  │     │
   │   │ (trait)          │   │  - cards table              │     │
   │   │  - create(card)  │   │  - leases table             │     │
   │   │  - claim(id,wkr) │   │  - deps table               │     │
   │   │  - heartbeat()   │   └─────────────────────────────┘     │
   │   │  - complete(id)  │                                       │
   │   │  - release(id)   │   ┌─────────────────────────────┐     │
   │   │  - list(filter)  │   │ ZombieReaper (tokio task)   │     │
   │   └──────────────────┘   │  every 30s:                 │     │
   │                          │   release stale leases      │     │
   │                          │   (lease_until < now)       │     │
   │                          └─────────────────────────────┘     │
   └──────────────────────────────────────────────────────────────┘
                          │
                          ▼
   ┌──────────────────────────────────────────────────────────────┐
   │              edgecrab-tools/kanban_tools.rs                  │
   │                                                              │
   │   kanban_create, kanban_claim, kanban_complete,              │
   │   kanban_release, kanban_list, kanban_block_on,              │
   │   kanban_heartbeat (auto-called by worker loop)              │
   └──────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-core/src/kanban/mod.rs` — trait + types |
| **New impl** | `crates/edgecrab-core/src/kanban/sqlite.rs` |
| **Migrations** | `crates/edgecrab-state/migrations/NNN_kanban.sql` (`cards`, `card_leases`, `card_deps`) |
| **Reaper** | `crates/edgecrab-core/src/kanban/reaper.rs` — tokio task spawned by `Agent::new` if board present |
| **Tools** | `crates/edgecrab-tools/src/tools/kanban_tools.rs` (one file, six tool impls) |
| **Slash commands** | `/kanban` + subcommands in `crates/edgecrab-cli/src/commands.rs` |
| **Builder wiring** | `AgentBuilder::kanban(Arc<dyn KanbanBoard>)` |

## Lease Semantics

- `claim(card_id, worker_id, lease_secs)` → exclusive lock with TTL.
- Worker MUST call `heartbeat(card_id, worker_id)` periodically (recommended
  every `lease_secs / 3`).
- If `lease_until < now()`, reaper releases the card to TODO; another
  worker may claim.

## Dependency Resolution

`claim()` refuses a card whose any `depends_on` parent is not DONE.
`list({ ready: true })` returns only cards with all deps done.

## Phasing

| Phase | Scope |
|-------|-------|
| 1 | Cards + create/claim/complete/list (no leases, no deps) |
| 2 | Leases + heartbeat + reaper |
| 3 | Dependencies + `block_on` |
| 4 | Worker auto-spawn via `delegate_task` integration |

## DRY / SOLID Notes

- **SRP:** `KanbanBoard` is storage; `Reaper` is lifecycle; tools are
  the LLM-facing surface. Three crates of concern, three modules.
- **DIP:** in-memory `KanbanBoard` impl exists for tests.
- **DRY:** card → goal one-liner uses `render_goal_block` from feature 001.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Cards effectively == goals: see [../001-persistent-goals/](../001-persistent-goals/)
