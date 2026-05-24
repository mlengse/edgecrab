# 022 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Tool registration | `inventory!` at module load — all schemas built eagerly |
| Model catalog | `crates/edgecrab-core/src/model_catalog.rs` — `OnceLock` eager merge |
| Skills summary | `crates/edgecrab-cli/src/` directory walk on every launch |
| Context files | `crates/edgecrab-core/src/prompt_builder.rs` — read + scan every launch |

## What Is Missing

1. Lazy schema construction (`OnceLock` per tool).
2. Background-thread catalog merge.
3. Skills summary cache file with mtime check.
4. Context-file content+scan cache.
5. `--profile-startup` flag.

## Honest Assessment

Rust is already fast. Gains here are 30–70ms — perceivable on slow
disks (HDD laptops, network-mounted home dirs) and on Termux/Android.
Disk caches make the second launch dramatically better.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
