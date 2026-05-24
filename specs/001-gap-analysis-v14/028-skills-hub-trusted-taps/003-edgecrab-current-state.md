# 028 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Skills hub | `crates/edgecrab-tools/src/tools/skills_hub.rs` |
| Skills guard | `crates/edgecrab-tools/src/tools/skills_guard.rs` (23 threat patterns) |
| Skills sync | `crates/edgecrab-tools/src/tools/skills_sync.rs` |
| Install command | `/skills install owner/repo/path` |

## What Is Missing

1. Tap concept — no notion of a curated registry.
2. Signature verification (Ed25519 / minisign / sigstore).
3. Publisher key store + TOFU pinning.
4. Manifest format.
5. CLI to manage taps (`add`, `remove`, `list`, `update`, `verify`).

## Honest Assessment

`skills_guard.rs` is necessary but insufficient. Trusted taps add a
*social* defence layer atop the technical one. Together they make the
ecosystem viable.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
