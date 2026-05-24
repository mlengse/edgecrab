# 025 — i18n / Localisation

**Tier:** C | **Impact:** 2 | **Value-per-Effort:** 2 | **Risk:** 1
**Primitive moved:** Accessibility (audience reach)

## Why It Matters (First Principles)

System prompts in English bias the model toward English output even
when the user writes in another language. Slash-command help and
error messages in English exclude non-English users. Hermes v0.14
ships 16 locales for system prompt + UI strings.

## The Gap

EdgeCrab is English-only.

## What EdgeCrab Gets Wrong Today

A French user writes "résume ce fichier" — the agent often replies in
English because every system prompt component is English. Slash
command help is in English. Error messages are in English.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
