# 006 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Checkpoint tool | `crates/edgecrab-tools/src/tools/checkpoint.rs` |
| `/rollback` command | `crates/edgecrab-cli/src/commands.rs` |

## What Is Missing

1. No count cap.
2. No size cap.
3. No FIFO eviction.
4. No hard-link Δ saving (storage cost = full copy × N).
5. No automatic exclude rules (`target/`, `node_modules/`, `.venv/`, `.git/`).
6. No SQLite-backed checkpoint index (probably JSON file?).

## Honest Assessment

The current checkpoint tool is **functionally a foot-gun** on any real
project. It needs the full v2 treatment before it can be safely enabled
by default.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
