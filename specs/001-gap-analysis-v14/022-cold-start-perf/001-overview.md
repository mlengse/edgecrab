# 022 — Cold-Start Performance

**Tier:** C | **Impact:** 2 | **Value-per-Effort:** 3 | **Risk:** 1
**Primitive moved:** Trust (perceived responsiveness)

## Why It Matters (First Principles)

The first 200ms of a CLI launch shapes user trust. Hermes v0.14 cut
cold-start by ~40% via lazy module loading, deferred catalog merge,
and async background warm-up. Every millisecond is felt.

## The Gap

EdgeCrab eagerly loads: full model catalog merge, every tool's
`schema()`, scans `~/.edgecrab/skills/` synchronously, reads all
context files (SOUL.md/AGENTS.md) on startup.

## What EdgeCrab Gets Wrong Today

Profiling reveals the first-paint to TTFB is dominated by:
1. Model catalog YAML parse + merge (~25ms).
2. Tool schema enumeration (~10ms).
3. Skills directory scan (~15ms for 30 skills).
4. AGENTS.md walk + injection scan (~10ms).

All four happen *before* the user sees the prompt.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
