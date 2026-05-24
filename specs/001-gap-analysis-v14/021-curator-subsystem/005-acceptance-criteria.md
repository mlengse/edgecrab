# 021 — Acceptance Criteria

## Functional

- [ ] After 20 memory writes, curator runs in background (verify with
      a fake "memory" file of 20+ bullets including duplicates).
- [ ] Duplicates collapsed in rewritten MEMORY.md.
- [ ] Archived entries land in `~/.edgecrab/memories/archive/`.
- [ ] `/curator status` shows last run, kept/merged/archived counts.
- [ ] `/curator revert` restores previous MEMORY.md from diff log.
- [ ] `/curator run` triggers an immediate run.

## Safety

- [ ] Atomic rewrite: kill -9 during apply → MEMORY.md remains valid
      (either old or new, never partial).
- [ ] Lock file prevents concurrent runs.
- [ ] Original always recoverable from archive log.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Curator subagent uses a strict JSON output schema; invalid plan
      → discard run, log error.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
