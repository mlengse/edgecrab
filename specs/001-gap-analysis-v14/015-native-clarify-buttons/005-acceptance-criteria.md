# 015 — Acceptance Criteria

## Functional

- [ ] `clarify({ question, options: [...] })` on Telegram renders an
      inline keyboard; tap delivers the option's `value` to the agent
      as if the user typed it.
- [ ] Discord renders `action_row` of buttons; interaction tap works.
- [ ] Slack renders block kit `actions`; tap works.
- [ ] SMS / generic webhook falls back to numbered text; user reply by
      number or by value text both accepted.
- [ ] Telegram `answer_callback_query` is called (so the tap "settles"
      on the client).

## Phased Delivery

- [ ] Phase 1: Telegram (highest user demand for buttons).
- [ ] Phase 2: Discord.
- [ ] Phase 3: Slack.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 10 tests; fallback path covered for at least one platform that
      doesn't implement `send_clarify`.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
