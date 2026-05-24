# 005 — Acceptance Criteria

## Functional

- [ ] `/handoff anthropic/claude-haiku-4` hot-swaps the active model.
- [ ] User sees a one-paragraph brief of the in-flight task before the
      next turn.
- [ ] Persistent goals (folder 001) survive the handoff.
- [ ] Conversation history is intact unless context-window forces
      compression — in which case the user is told.
- [ ] Auth failure for the target provider produces a clean error and
      leaves the session on the original model.
- [ ] Gateway users can `/handoff` from Telegram, Slack, Discord.
- [ ] `/insights` lists handoffs that occurred this session.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `cargo test -p edgecrab-core handoff::` ≥ 6 tests:
      target unknown, smaller window + compress OK, smaller window +
      compress fail, brief generation OK, brief generation fallback,
      auth failure leaves state intact.
- [ ] No mutation of `cached_system_prompt`; new provider gets a fresh
      `SystemPromptBlocks`.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
