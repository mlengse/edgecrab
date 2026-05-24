# 007 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `delegate_task` tool | `crates/edgecrab-tools/src/tools/delegate_task.rs` |
| `todo` tool | `crates/edgecrab-tools/src/tools/todo.rs` |
| Sub-agent runner | `crates/edgecrab-core/src/sub_agent_runner.rs` |
| Session DB | `crates/edgecrab-state/` |

## What Is Missing

1. Durable kanban table (would land in `edgecrab-state`).
2. Lease + heartbeat model.
3. Zombie reclaim daemon (per-process background task).
4. New tools: `kanban_create`, `kanban_claim`, `kanban_release`,
   `kanban_complete`, `kanban_list`, `kanban_block_on`.
5. Slash command surface: `/kanban`, `/kanban add`, `/kanban list`.

## Honest Assessment

This is the largest single build in Tier A. Recommended sequencing:
ship cards + claim + complete first (no worker spawning); add lease/
heartbeat/zombie in a phase 2; add DAG dependencies in phase 3.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
