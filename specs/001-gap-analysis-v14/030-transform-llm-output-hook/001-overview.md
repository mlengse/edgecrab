# 030 — Transform-LLM-Output Plugin Hook

**Tier:** C | **Impact:** 2 | **Value-per-Effort:** 3 | **Risk:** 2
**Primitive moved:** Extensibility (output transformation)

## Why It Matters (First Principles)

Plugins can already wrap tools (folder 009 `tool_override`). The
symmetric capability — wrap the LLM's *output* before it reaches the
user — is the missing half. Hermes v0.14 added a `transform_output`
hook used for inline citation linking, profanity filtering,
localisation post-processing, and watermarking.

## The Gap

EdgeCrab has no plugin hook for the LLM's output.

## What EdgeCrab Gets Wrong Today

If a user wants to (e.g.) auto-convert every `crate::foo` reference
into a clickable docs.rs link, they must fork the renderer.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Mirrors `tool_override` from: [../009-pluggable-providers-plugins/](../009-pluggable-providers-plugins/)
