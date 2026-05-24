# 006 — Implementation Plan

## Architecture (ASCII)

```
   ┌────────────────────────────────────────────────────────────────┐
   │              edgecrab-tools/checkpoint                         │
   │                                                                │
   │   ┌──────────────────────────┐   ┌──────────────────────┐      │
   │   │ CheckpointStore (trait)  │──►│ FsCheckpointStore     │      │
   │   │  - save(workspace)       │   │  ~/.edgecrab/         │      │
   │   │  - list(session)         │   │   checkpoints/<sid>/  │      │
   │   │  - restore(ckpt_id)      │   │  - hard-link Δ        │      │
   │   │  - prune()               │   │  - exclude rules      │      │
   │   └────────────┬─────────────┘   └──────────────────────┘      │
   │                │                                                │
   │                ▼                                                │
   │   ┌──────────────────────────┐                                  │
   │   │ CheckpointIndex (SQLite) │                                  │
   │   │   (session_id, seq,      │                                  │
   │   │    bytes, file_count,    │                                  │
   │   │    created_at)           │                                  │
   │   └──────────────────────────┘                                  │
   └────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Refactor** | `crates/edgecrab-tools/src/tools/checkpoint.rs` — split into module: `mod.rs`, `store.rs`, `fs_store.rs`, `index.rs`, `excludes.rs` |
| **New trait** | `CheckpointStore { save, list, restore, prune }` |
| **Default impl** | `FsCheckpointStore` with hard-link Δ semantics (`std::fs::hard_link`) |
| **Index** | `CheckpointIndex` using the existing `edgecrab-state` SQLite connection (new migration) |
| **Excludes** | Default deny list: `target/`, `node_modules/`, `.git/`, `.venv/`, `__pycache__/`, `dist/`, `build/`, `*.lock` plus `.gitignore` honour |
| **Config** | `checkpoint.max_per_session: 20`, `checkpoint.max_mb_per_session: 200`, `checkpoint.excludes: [...]` |
| **Mutation footer integration** | Restore emits a `MutationRecord` per changed file → composes with feature 002 |

## DRY / SOLID Notes

- **SRP:** the store handles bytes; the index handles metadata; excludes
  are a separate pure module. Three small modules > one 500-line file.
- **DIP:** the tool depends on `CheckpointStore` trait, not on
  `FsCheckpointStore` directly. Future S3/git-based stores plug in.
- **OCP:** new exclude rules added via config, no code change.
- **DRY:** restore uses the same `MutationBuffer` channel as ordinary
  file writes — single source of truth for "what changed."

## Eviction Algorithm

```
On save(workspace) -> Result<CheckpointId>:
    write new checkpoint with hard-links
    update index
    loop:
        if count > max_per_session OR total_bytes > max_mb_per_session * 1MB:
            evict oldest non-pinned checkpoint
            continue
        break
```

Optional pinning: `/rollback pin <n>` marks a checkpoint immune to eviction.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
