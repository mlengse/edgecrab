# 006 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Checkpoint manager | `hermes-agent/tools/checkpoint_manager.py` (541 lines — substantial) |
| Slash command | `hermes-agent/hermes_cli/commands.py` (`/rollback`) |
| Storage layout | `~/.hermes/checkpoints/<session_id>/<seq>/` |
| Exclude rules | Built-in deny list + `.gitignore`-style overrides |
| Eviction policy | FIFO under count cap (`checkpoint.max_per_session`) and size cap (`checkpoint.max_mb_per_session`) |
| Restore | Atomic rename of working copy aside, then copy from checkpoint, then emit a mutation record |

## Mechanism

```
Per checkpoint save:
  1. Compute file manifest of tracked workspace (filtered)
  2. Hard-link unchanged files from previous checkpoint (Δ-saving)
  3. Copy changed files
  4. Update SQLite checkpoint index
  5. Run prune_if_over_limits()
```

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
