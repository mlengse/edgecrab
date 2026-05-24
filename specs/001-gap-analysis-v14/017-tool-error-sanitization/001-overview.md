# 017 — Tool-Error Sanitisation Layer

**Tier:** B | **Impact:** 3 | **Value-per-Effort:** 5 | **Risk:** 1
**Primitive moved:** Trust + Security (data hygiene)

## Why It Matters (First Principles)

When a tool fails, its error message is fed back to the LLM verbatim.
That error often contains: API keys, file paths, stack traces, internal
hostnames, user tokens. The LLM then *quotes them back to the user*,
sometimes in a streaming response that lands in chat history (and chat
logs, and analytics). Hermes v0.14 added a sanitisation layer that
runs every tool error through a redactor before the LLM ever sees it.

## The Gap

EdgeCrab has `redaction` for assistant *output* before display, but
**tool errors are not redacted** before being returned to the LLM
loop. A failing `terminal` command can leak the full env (`env: AWS_*`,
`OPENAI_API_KEY=sk-...` in error messages).

## What EdgeCrab Gets Wrong Today

A `file_read` error like `Permission denied: /Users/alice/.ssh/id_rsa`
goes straight into context. The LLM may surface "I can't read your SSH
key at /Users/alice/.ssh/id_rsa" in the final answer — leaking the
user's home directory, OS, and file structure to chat history.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
