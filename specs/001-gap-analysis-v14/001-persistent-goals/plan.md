# 001 — Persistent Goals — Implementation Plan (executed)

## Branch

`feat/persistent-goals` (from `feat/edgequake-v0.10.0`)

## Architecture

```
CLI / Gateway slash commands
        │
        ▼
Agent::goal_set / subgoal_push / subgoal_done / goal_show / goal_clear
        │
        ▼
GoalStore trait ──► SqliteGoalStore ──► SessionDb (session_goals + session_subgoals)
        │
        ▼
execute_loop: render_goal_block() → ephemeral user Message appended to API payload only
```

## Files Changed

| Action | Path |
|--------|------|
| New | `crates/edgecrab-core/src/goals/mod.rs` — trait, in-memory impl, render |
| New | `crates/edgecrab-core/src/goals/sqlite.rs` — SQLite adapter |
| Modified | `crates/edgecrab-state/src/schema.sql` — v7 goal tables |
| Modified | `crates/edgecrab-state/src/session_db.rs` — CRUD + migration |
| Modified | `crates/edgecrab-core/src/agent.rs` — store wiring + public API |
| Modified | `crates/edgecrab-core/src/conversation.rs` — per-turn injection |
| Modified | `crates/edgecrab-cli/src/commands.rs` — slash commands |
| Modified | `crates/edgecrab-cli/src/app.rs` — handlers |
| Modified | `crates/edgecrab-gateway/src/run.rs` — gateway dispatch |
| Modified | `crates/edgecrab-command-catalog/src/lib.rs` — catalog entries |
| Modified | `AGENTS.md` — documentation |

## Design Decisions

1. **Cache safety:** Goal block appended to a cloned message list for the API call only; `session.messages` and `cached_system_prompt` are untouched.
2. **Compression survival:** Goals live in SQLite, not in `Vec<Message>` — compression cannot erase them.
3. **ISP:** `GoalStore` has exactly 5 methods.
4. **DRY:** `render_goal_block()` shared by injection, `/goal show`, and tests.
5. **Fallback:** `InMemoryGoalStore` when no `state_db` (unit tests, minimal runs).

## Test Matrix

| Test | Location |
|------|----------|
| Empty store, set, push/pop, isolation, JSON, render | `goals::tests` (9) |
| SQLite persistence + isolation | `edgecrab-state::session_db::tests` (2) |
| Ephemeral injection | `conversation::tests::execute_loop_injects_goal_*` (2) |
| System prompt unchanged on `/goal` | `agent::tests::goal_set_does_not_mutate_cached_system_prompt` |

## Verification Commands

```bash
cargo test -p edgecrab-core goal
cargo test -p edgecrab-state goals_
cargo clippy --workspace -- -D warnings
cargo test --workspace
```
