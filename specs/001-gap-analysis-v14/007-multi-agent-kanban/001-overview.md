# 007 — Multi-Agent Kanban (Durable Board + Workers)

**Tier:** A | **Impact:** 5 | **Value-per-Effort:** 3 | **Risk:** 4
**Primitive moved:** Reliability of long-horizon execution

## Why It Matters

The Hermes v0.13 Kanban subsystem turns the agent into a **fleet manager**:
durable cards (TODO/DOING/DONE), worker agents that lease cards with
heartbeats, automatic zombie reclaim, and inter-card dependencies. This is
the qualitative leap from "one agent" to "small swarm" — for codebase-wide
refactors, large research tasks, or multi-PR campaigns.

## The Gap

EdgeCrab has:

- `delegate_task` tool (subagent delegation) — fire-and-forget single task.
- `todo` tool — flat list, no worker model.

EdgeCrab does **not** have:

- Durable card store across restarts.
- Worker leases + heartbeats.
- Zombie detection / reclaim.
- Card dependencies (DAG).
- Per-card streaming back to the supervising agent.

## What EdgeCrab Gets Wrong Today

`delegate_task` works for "do this one thing" but cannot orchestrate
"refactor these 18 packages in parallel with a budget of 3 concurrent
workers and a dependency graph." Users hit this wall fast.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Composes with: [001-persistent-goals/](../001-persistent-goals/) (one card == one goal)
