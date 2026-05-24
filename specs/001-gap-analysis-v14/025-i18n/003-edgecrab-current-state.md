# 025 — EdgeCrab Current State

| Existing | File |
|----------|------|
| English strings | hardcoded throughout `edgecrab-core`, `edgecrab-cli`, `edgecrab-tools` |
| System prompt | `crates/edgecrab-core/src/prompt_builder.rs` — `DEFAULT_IDENTITY`, `MEMORY_GUIDANCE`, etc. constants |
| Slash command help | `crates/edgecrab-cli/src/commands.rs` |
| Error messages | scattered `ToolError` variants |

## What Is Missing

1. No i18n module / catalog.
2. No language selector.
3. No `t!()`-style lookup macro.
4. All strings inline.

## Honest Assessment

The mechanical effort is enormous (hundreds of strings to extract).
The high-value subset (system prompt + setup wizard + main errors)
is tractable. Phase: extract high-value strings first; tools/help
later.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
