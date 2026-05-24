# 001 — Acceptance Criteria

A reviewer can mark this feature **done** when **all** of the following pass:

## Functional

- [ ] `/goal "Refactor X"` persists across CLI restarts and gateway restarts.
- [ ] `/goal show` lists the active goal and all subgoals.
- [ ] `/subgoal "step 1"` then `/subgoal "step 2"` produces a 2-item stack.
- [ ] `/done` pops the most-recently-pushed subgoal and marks it `[x]`.
- [ ] `/goal clear` empties everything for the current session only.
- [ ] After `/compress`, the goal block is still injected next turn
      (proven by inspecting the message sent to the provider).
- [ ] Two concurrent sessions in the same gateway have independent goal stacks.

## Cache Safety

- [ ] Anthropic `cache_creation_input_tokens` is `0` on turn N+1 after
      `/goal` is set on turn N, *provided* the underlying system prompt
      did not change. Verified via `/cost` output.
- [ ] System prompt is **not** mutated when a goal is added (assert via
      `SessionState::cached_system_prompt` unchanged hash).

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] `cargo test -p edgecrab-core goals::` passes ≥ 8 tests
      covering: empty store, set, push/pop ordering, two-session
      isolation, JSON round-trip, render block, compression survival,
      cache hash unchanged.
- [ ] `GoalStore` trait has ≤ 6 methods (ISP guard).
- [ ] No new `unwrap()` in `goals/sqlite.rs`.

## Documentation

- [ ] `AGENTS.md` Slash Commands table lists `/goal`, `/subgoal`, `/done`.
- [ ] `AGENTS.md` adds a "Persistent Goals" subsection under Agent Architecture.

## Cross-References

- Overview: [001-overview.md](001-overview.md)
- Implementation: [004-implementation-plan.md](004-implementation-plan.md)
