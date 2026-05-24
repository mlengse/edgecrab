# 003 — LSP Semantic Diagnostics on `write_file` / `patch_file`

**Tier:** S | **Impact:** 5 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Trust in side-effects

## Why It Matters

Syntactically valid code can still be semantically broken (undefined symbol,
wrong type, missing import). Hermes v0.13 wired its LSP layer to run a
**post-write semantic diagnostic** on every mutated file — the agent sees
`rust-analyzer` errors *in the same turn* and can self-correct before the
user even reads the message.

EdgeCrab already has `crates/edgecrab-lsp/` — it's the **easiest big win**
in the entire gap analysis because the hard part (LSP client) is done.

## The Gap

`edgecrab-lsp` exists but is **not wired to file mutation tools**.
`file_write` and `file_patch` return success without ever consulting the
LSP for semantic errors.

## What EdgeCrab Gets Wrong Today

The agent writes broken code → user runs `cargo check` → user pastes error
back → wasted turn. With LSP-on-write the agent catches the error the same
turn it makes it, often self-fixes in the next tool call, and the user
never sees the bad version.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Composes with: [../002-file-mutation-verifier/](../002-file-mutation-verifier/)
