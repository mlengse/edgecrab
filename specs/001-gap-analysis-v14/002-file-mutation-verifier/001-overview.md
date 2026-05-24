# 002 — Per-Turn File-Mutation Verifier Footer

**Tier:** S | **Impact:** 5 | **Value-per-Effort:** 5 | **Risk:** 1
**Primitive moved:** Trust in side-effects

## Why It Matters

The #1 source of agent regret is "the agent said it edited the file but it
didn't / edited the wrong one." Hermes v0.14 added a **verifier footer**
appended to every assistant turn that touched the filesystem:

```
─── files-mutated this turn ───────────────────────
M  crates/edgecrab-core/src/agent.rs       +12 −3
A  crates/edgecrab-core/src/goals/mod.rs   +84
D  legacy/old.rs                            −210
───────────────────────────────────────────────────
```

The user — and the model itself on the *next* turn — sees ground truth.
This closes the hallucination loop without any extra LLM tokens.

## The Gap

EdgeCrab's `file_write`, `file_patch`, `file_read` tools return JSON
results to the model only. The user sees streamed token output and a
tool-prefix indicator — but no consolidated mutation log per turn.

## What EdgeCrab Gets Wrong Today

You can scroll back through 80 turns and have no idea which files changed,
in what order, with what net diff. The model on turn N+1 has no
authoritative view of its own past edits beyond what it remembers from
tool-result echoes.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Related: [003-lsp-write-diagnostics/](../003-lsp-write-diagnostics/)
- Related: [006-checkpoints-v2/](../006-checkpoints-v2/)
