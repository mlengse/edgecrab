# 006 — Checkpoints v2 Implementation Plan

## Approach

Port Hermes Agent v2 `CheckpointManager` (`tools/checkpoint_manager.py`) to Rust as a
module under `crates/edgecrab-tools/src/tools/checkpoint/`. Hermes uses a **single shared
bare git store** with per-project refs — not per-project shadow repos or hard-link FS
snapshots. EdgeCrab v1 used per-project shadow repos with broken rebase pruning; v2
aligns with Hermes Code.

## Module Layout

| File | Responsibility |
|------|----------------|
| `mod.rs` | Tool handler, `ensure_checkpoint`, `checkpoint_new_turn` |
| `manager.rs` | `CheckpointManager` — save, list, restore, pin, prune ref, size cap |
| `git.rs` | Git env isolation, path helpers, project metadata |
| `excludes.rs` | `DEFAULT_EXCLUDES` → `store/info/exclude` |
| `prune.rs` | Startup `maybe_auto_prune_checkpoints`, orphan/stale sweep |
| `tests.rs` | 14+ unit/integration tests |

## Config (`checkpoints:` in config.yaml)

```yaml
checkpoints:
  enabled: true
  max_snapshots: 20          # FIFO ref rewrite + gc (was 50, unenforced)
  max_total_size_mb: 200     # global store cap (Hermes default 500)
  max_file_size_mb: 10       # skip oversize files when staging
  auto_prune: true
  retention_days: 7
  delete_orphans: true
  min_interval_hours: 24
```

## Wiring

- `conversation.rs`: `checkpoint_new_turn()` each ReAct iteration
- `main.rs`: `maybe_auto_prune_checkpoints()` at startup when `auto_prune: true`
- `file_write` / `file_patch` / `terminal` / LSP: unchanged `ensure_checkpoint()` API
- Restore emits `MutationRecord` via `ToolContext.record_mutation()` (feature 002)

## Edge Cases

| Case | Mitigation |
|------|------------|
| Git missing | Silent skip (debug log) |
| Root/home cwd | Skip snapshot (too broad) |
| >50k files | Skip snapshot |
| Oversize single file | Dropped from index before commit |
| Crash mid-write | Startup prune + gc reclaims orphans |
| Path traversal on restore_file | `validate_file_path` rejects `..` |
| Commit hash injection | `validate_commit_hash` rejects `-` prefix |
| Legacy v1 repos | Auto-migrate to `legacy-<timestamp>/` |
| Pinned checkpoint | Stored in `projects/<hash>.json`; survives FIFO eviction |

## Deferred (not in Hermes either)

- SQLite checkpoint index (Hermes uses git refs + JSON metadata; sufficient)
- Hard-link Δ FS store (Hermes uses git object dedup instead — equivalent goal)
- Direct `/rollback` TUI RPC (still agent-mediated; list JSON includes `size_bytes`)
