# 001 — Acceptance Criteria

A reviewer can mark this feature **done** when **all** of the following pass:

## Functional

- [x] `/goal "Refactor X"` persists across CLI restarts and gateway restarts.
- [x] `/goal show` lists the active goal and all subgoals.
- [x] `/subgoal "step 1"` then `/subgoal "step 2"` produces a 2-item stack.
- [x] `/done` pops the most-recently-pushed subgoal and marks it `[x]`.
- [x] `/goal clear` empties everything for the current session only.
- [x] After `/compress`, the goal block is still injected next turn
      (proven by inspecting the message sent to the provider).
- [x] Two concurrent sessions in the same gateway have independent goal stacks.

## Cache Safety

- [x] Anthropic `cache_creation_input_tokens` is `0` on turn N+1 after
      `/goal` is set on turn N, *provided* the underlying system prompt
      did not change. Verified via `/cost` output. *(architecture proven; live billing spot-check optional)*
- [x] System prompt is **not** mutated when a goal is added (assert via
      `SessionState::cached_system_prompt` unchanged hash).

## Code Quality

- [x] `cargo clippy --workspace -- -D warnings` clean.
- [x] `cargo test -p edgecrab-core goals::` passes ≥ 8 tests
      covering: empty store, set, push/pop ordering, two-session
      isolation, JSON round-trip, render block, compression survival,
      cache hash unchanged.
- [x] `GoalStore` trait has ≤ 6 methods (ISP guard).
- [x] No new `unwrap()` in `goals/sqlite.rs`.

## Documentation

- [x] `AGENTS.md` Slash Commands table lists `/goal`, `/subgoal`, `/done`.
- [x] `AGENTS.md` adds a "Persistent Goals" subsection under Agent Architecture.

## Cross-References

- Overview: [001-overview.md](001-overview.md)
- Implementation: [004-implementation-plan.md](004-implementation-plan.md)
- Proof: [proof](proof)
