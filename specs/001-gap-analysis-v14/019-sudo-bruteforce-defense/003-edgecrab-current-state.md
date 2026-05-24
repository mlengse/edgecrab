# 019 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `terminal` tool | `crates/edgecrab-tools/src/tools/terminal.rs` |
| Command scanner | `crates/edgecrab-security/src/command_scan.rs` |
| `--yolo` flag | global agent config |

## What Is Missing

1. No `sudo`-specific interception.
2. No session-level brute-force counter.
3. No mode selector (`block` / `confirm` / `allow`).
4. No allowlist evaluation.
5. No `sudo -S` stdin scrutiny.

## Honest Assessment

`command_scan.rs` checks for shell-injection metacharacters but not for
intent-specific patterns like privileged escalation. Sudo deserves its
own gate.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
