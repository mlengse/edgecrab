# 022 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │   Launch sequence (revised)                                      │
   │                                                                  │
   │   t=0   parse argv (no IO)                                       │
   │   t=1   spawn background tasks (3, in parallel):                 │
   │            ┌─ load catalog overrides + merge into Arc<Catalog>   │
   │            ├─ scan skills dir → write/refresh .cache.json        │
   │            └─ walk context files; for each:                      │
   │                 if (path,mtime,size) in scan_cache → use cached  │
   │                 else inject-scan + cache                         │
   │   t=2   render initial TUI frame using:                          │
   │            - embedded default catalog (if overrides not ready)   │
   │            - skills .cache.json (if exists from last launch)     │
   │            - empty system prompt placeholder                     │
   │   t=3   await background tasks → build final system prompt       │
   │   t=4   user types → first send uses final prompt                │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Lazy tool schemas** | refactor `crates/edgecrab-tools/src/registry.rs` — each tool wraps `schema()` in `OnceLock`; collection iterates names eagerly, schemas lazily |
| **Catalog background merge** | `crates/edgecrab-core/src/model_catalog.rs` — split `get_embedded()` vs `get_full()`; spawn merge task |
| **Skills cache** | `~/.edgecrab/skills/.cache.json` (path, mtime, summary); refreshed in background; freshness check: if any skill file mtime > cache mtime, refresh |
| **Context-file cache** | `~/.edgecrab/.context_cache.json` (path → {mtime,size,scan_verdict,injection_blocked}); reused if unchanged |
| **Startup profiler** | `--profile-startup` instruments each phase with `Instant::now()` deltas, prints sorted breakdown |
| **Tests** | benchmark harness measuring TTFP (time to first paint) on a fixture `~/.edgecrab` with many skills + AGENTS.md |

## Risks

- Background tasks must finish before first message send (or fall back
  to embedded catalog). Race conditions if user types immediately —
  handle with `await` at message-send boundary, not at TUI startup.
- Cache invalidation: file modified externally between launches →
  mtime check catches it. Symlinks: stat the symlink target.

## DRY / SOLID Notes

- **SRP:** each cache is its own struct with read/write/invalidate.
- **OCP:** new cache types follow the same `MtimeCache<T>` generic.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
