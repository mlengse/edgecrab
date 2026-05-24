# 001 — EdgeCrab Current State

## What Exists

| EdgeCrab capability | File | Gap |
|---------------------|------|-----|
| Mission Steering (HINT/REDIRECT/STOP) | `crates/edgecrab-core/src/agent.rs` | One-shot; not persistent |
| Steering channel `steer_sender()` | `crates/edgecrab-core/src/agent.rs` | Drained once per ReAct boundary, then gone |
| `/queue` next-turn enqueue | `crates/edgecrab-gateway/src/run.rs` (`second_message_mode: queue`) | Fires once |
| Slash command registry | `crates/edgecrab-cli/src/commands.rs` | No `/goal`, `/subgoal`, `/done` |
| Conversation history | `Vec<Message>` in `SessionState` | Goals would be lost in `/compress` |

## What Is Missing

1. **No persistent goal store.** Goals would need a new struct in
   `edgecrab-core` (or a new `edgecrab-goals` crate) reachable from both
   CLI and gateway.
2. **No per-turn re-injection hook.** `conversation.rs::execute_loop()`
   builds the message list once and reuses it. There is no extension point
   to append a synthetic user message *before each LLM call*.
3. **No `/compress` survival.** `compression.rs` drops all messages
   except the last N + a summary block. Goals would be erased.
4. **No slash commands.** `commands.rs` `CommandResult` enum does not have
   `GoalSet`, `SubgoalPush`, `Done` variants.

## Honest Assessment

EdgeCrab's steering primitive is *technically* sufficient to fake persistent
goals from the gateway side by re-sending a HINT each turn — but that:

- Doubles per-turn token cost (HINT text re-sent every turn unbatched).
- Loses the goal on `/compress`.
- Requires every CLI/gateway path to re-implement the loop.

The right fix is core: a `GoalStore` trait + `PromptBuilder` integration +
slash command wiring.

## Cross-References

- Overview: [001-overview.md](001-overview.md)
- Hermes reference: [002-hermes-reference.md](002-hermes-reference.md)
- Implementation: [004-implementation-plan.md](004-implementation-plan.md)
