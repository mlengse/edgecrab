# 001 — Hermes Reference

## Where It Lives in Hermes

Persistent goals are a **plugin** in Hermes, not a core module — which is
itself a design choice EdgeCrab should consider mirroring (extensibility >
core bloat).

| Concern | Hermes file |
|---------|-------------|
| Goal stack data structure | `hermes-agent/plugins/` (goal plugin module) |
| Slash command registration | `hermes-agent/hermes_cli/commands.py` (`CommandDef` entries via plugin hook) |
| Per-turn injection | `hermes-agent/agent/prompt_builder.py` (`build()` re-renders user message with active goals) |
| Subgoal pop on `/done` | plugin event handler |
| Cache-safe append | uses fresh `user` message (not system-prompt mutation) to preserve Anthropic 1h cache |

## Data Model (inferred from release notes)

```
Goal {
    id: str,           // "g_2025-05-24_a1b2"
    text: str,         // "Refactor payment service to async/await"
    created_at: iso8601,
    pinned: bool,      // /goal pins to top of every prompt
    subgoals: [SubGoal],
}

SubGoal {
    id: str,
    text: str,
    parent_goal_id: str,
    done: bool,
}
```

## Slash Commands

| Command | Effect |
|---------|--------|
| `/goal <text>` | Replace current top-level goal |
| `/goal show`   | Display active goals |
| `/goal clear`  | Wipe all goals |
| `/subgoal <text>` | Push subgoal onto current goal's stack |
| `/done`        | Pop top subgoal (mark done) |

## Per-Turn Injection (verbatim concept)

Before each `provider.chat(...)`, Hermes appends a synthetic `user` message:

```
[GOAL CONTEXT — auto-injected each turn]
Active goal: Refactor payment service to async/await
Subgoals:
  1. [x] migrate handlers
  2. [ ] update tests
  3. [ ] benchmark p95
```

Crucially this is a **user-role** message, not a system-prompt mutation, so
Anthropic's prompt cache stays warm.

## Cross-References

- Overview: [001-overview.md](001-overview.md)
- Implementation: [004-implementation-plan.md](004-implementation-plan.md)
- Cache-safety related: [../004-prompt-prefix-cache/](../004-prompt-prefix-cache/)
