# 021 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Memory tool | `crates/edgecrab-tools/src/tools/memory.rs` |
| Memory files | `~/.edgecrab/memories/{MEMORY.md, USER.md}` |
| Sub-agent runner | `crates/edgecrab-core/src/sub_agent_runner.rs` |

## What Is Missing

1. No background curator.
2. No write-counter / time trigger.
3. No archive directory.
4. No diff log / revert.

## Honest Assessment

Sub-agent runner already exists. Curator is a specialised sub-agent
with one input (memory file) and one structured output (rewrite plan
+ kept/archived lists). Build cost: one prompt template, a trigger,
and a CLI revert.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
