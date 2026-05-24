# 006 — Checkpoints v2 (Pruning + Disk Guardrails)

**Tier:** S | **Impact:** 4 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Trust in side-effects

## Why It Matters

Filesystem checkpoints let users undo a bad agent edit-spree.
Hermes v0.13 elevated checkpoints from "snapshot files on demand"
to a **bounded, auto-pruned, disk-aware** subsystem:

- Max N checkpoints per session (default 20).
- Max M MB on disk per session (default 200 MB) — older checkpoints
  evicted FIFO when exceeded.
- Excludes `target/`, `node_modules/`, `.git/`, `.venv/` automatically.
- Restores are atomic (rename-based) and produce a mutation footer
  (composes with feature 002).

## The Gap

EdgeCrab has the `checkpoint` tool (`crates/edgecrab-tools/src/tools/checkpoint.rs`)
and `/rollback` slash command but **no enforcement of pruning or disk caps**.
A 4-hour coding session can balloon to gigabytes of checkpoints.

## What EdgeCrab Gets Wrong Today

Users either:
- Disable checkpoints entirely after one full-disk incident, OR
- Keep them on and silently leak disk until a system crash.

Neither is acceptable.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Composes with: [002-file-mutation-verifier/](../002-file-mutation-verifier/)
