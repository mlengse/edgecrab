# 001 — Implementation Plan

## Architecture (ASCII)

```
                                ┌──────────────────────────────┐
                                │      edgecrab-core           │
                                │                              │
   ┌────────────┐               │   ┌──────────────────────┐   │
   │  CLI       │  set/push     │   │  GoalStore trait     │   │
   │  /goal     │──────────────►│   │   - active() -> Vec  │   │
   │  /subgoal  │               │   │   - set(Goal)        │   │
   │  /done     │               │   │   - push_subgoal()   │   │
   └────────────┘               │   │   - pop_subgoal()    │   │
                                │   └──────────┬───────────┘   │
   ┌────────────┐  set/push     │              │               │
   │  Gateway   │──────────────►│   ┌──────────▼───────────┐   │
   │  /goal     │               │   │ SqliteGoalStore impl │   │
   └────────────┘               │   │ (per session_id)     │   │
                                │   └──────────────────────┘   │
                                │              │               │
                                │              │ active goals  │
                                │              ▼               │
                                │   ┌──────────────────────┐   │
                                │   │  conversation.rs     │   │
                                │   │  execute_loop:       │   │
                                │   │    before each       │   │
                                │   │    provider.chat:    │   │
                                │   │      inject_goals()  │   │
                                │   │      as USER msg     │   │
                                │   │      (cache-safe)    │   │
                                │   └──────────────────────┘   │
                                └──────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New trait** | `crates/edgecrab-core/src/goals/mod.rs` — `GoalStore`, `Goal`, `SubGoal` |
| **New impl** | `crates/edgecrab-core/src/goals/sqlite.rs` — `SqliteGoalStore` (uses existing `edgecrab-state` connection) |
| **State migration** | `crates/edgecrab-state/migrations/NNN_goals.sql` — `goals` + `subgoals` tables keyed by `session_id` |
| **Loop integration** | `crates/edgecrab-core/src/conversation.rs` — call `inject_goals(messages, store)` right after compression check |
| **Builder wiring** | `crates/edgecrab-core/src/agent.rs` — `AgentBuilder::goal_store(Arc<dyn GoalStore>)` |
| **Slash commands (CLI)** | `crates/edgecrab-cli/src/commands.rs` — add `GoalSet`, `GoalShow`, `GoalClear`, `SubgoalPush`, `SubgoalDone` to `CommandResult` |
| **Slash commands (gateway)** | `crates/edgecrab-gateway/src/run.rs` — dispatch same variants |
| **Compression survival** | `crates/edgecrab-core/src/compression.rs` — goals are stored *outside* the message vec, so compression is naturally goal-safe |

## DRY / SOLID Notes

- **SRP:** `GoalStore` does storage only; `inject_goals(messages, store)` is a
  pure free function in `conversation.rs` — no method on the store.
- **OCP:** new storage backends (in-memory, postgres) implement `GoalStore`
  without touching the loop.
- **ISP:** `GoalStore` has 5 methods; resist the urge to add `audit_log()`
  or `export()` here — those belong on a separate trait.
- **DRY:** the rendering function `render_goal_block(&[Goal]) -> String`
  lives in `goals/mod.rs` and is reused by CLI `/goal show` and by the
  loop injector.

## Cache Safety (critical)

The injected goal block MUST be a **user-role message**, appended to
`messages` immediately before `provider.chat(...)`. Never mutate the
cached system prompt — see [../004-prompt-prefix-cache/004-implementation-plan.md](../004-prompt-prefix-cache/004-implementation-plan.md).

```rust
// in execute_loop, after compression check:
let goal_block = render_goal_block(&goal_store.active(session_id).await?);
if !goal_block.is_empty() {
    messages.push(Message::user(goal_block)); // ephemeral; not persisted
}
let response = provider.chat(model, &messages, tools).await?;
// pop the ephemeral goal block before persistence:
messages.pop();
```

## Cross-References

- Overview: [001-overview.md](001-overview.md)
- Acceptance: [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Cache safety: [../004-prompt-prefix-cache/](../004-prompt-prefix-cache/)
