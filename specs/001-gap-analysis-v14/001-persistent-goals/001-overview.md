# 001 — Persistent Goals (`/goal` Ralph Loop + `/subgoal`)

**Tier:** S | **Impact:** 5 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Reliability of long-horizon execution

## Why It Matters (First Principles)

LLMs lose the plot after ~30 turns. Token-window compression preserves *facts*
but not *intent*. Hermes v0.14 introduced a **persistent goal** that is
re-injected into the system prompt on every turn — the so-called "Ralph loop"
(after the agent that just keeps grinding on the same task). Subgoals stack
underneath, are auto-popped when satisfied, and survive `/compress`.

This is the single highest-leverage feature in v0.14. It turns EdgeCrab from
a chat companion into a **mission-running** agent.

## The Gap

EdgeCrab has:

- `/queue` (next-turn enqueue) — fires once.
- Mission Steering (HINT/REDIRECT/STOP) — *injected once* at the next loop boundary.

EdgeCrab does **not** have:

- A persistent goal string re-rendered into the system prompt every turn.
- A subgoal stack with explicit `/subgoal` / `/done` slash commands.
- Cache-safe re-rendering (Hermes carefully appends goals as a fresh user
  message, not a system-prompt mutation, to preserve Anthropic prompt cache).

## What EdgeCrab Gets Wrong Today

Mission Steering is a **one-shot bandage**. The user must keep re-injecting
intent manually. There is no mechanism for "stay on goal X until I say
otherwise." This is the difference between a chatbot and an agent.

## Cross-References

- Hermes reference: [002-hermes-reference.md](002-hermes-reference.md)
- EdgeCrab current state: [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- Implementation plan: [004-implementation-plan.md](004-implementation-plan.md)
- Acceptance criteria: [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Related: [005-session-handoff/](../005-session-handoff/) (handoff carries goals across models)
- Related: [007-multi-agent-kanban/](../007-multi-agent-kanban/) (Kanban cards == goals at scale)
