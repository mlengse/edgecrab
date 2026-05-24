# 015 — Native Clarify Buttons (Telegram/Discord Inline Keyboards)

**Tier:** B | **Impact:** 3 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Trust (clarification UX) + Reach (mobile-first users)

## Why It Matters (First Principles)

The `clarify` tool asks the user a question. In the TUI, this renders
as text. On Telegram/Discord/Slack, **text-only clarification is awful
mobile UX**: the user has to retype an option exactly. Native inline
keyboards / button rows let the user tap once.

Hermes v0.14 added per-platform `clarify` rendering: the same tool
schema, but the platform adapter chooses between text, button row,
or modal based on capabilities.

## The Gap

EdgeCrab's `clarify` tool emits text. No platform adapter intercepts it
to render buttons. The user must retype "yes" / "option 2" / etc.

## What EdgeCrab Gets Wrong Today

Telegram inline keyboards are trivial (just a JSON array on
`sendMessage`); Discord has components v2 (`action_row`); Slack has
block kit `actions`. We're leaving easy mobile UX on the table.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
