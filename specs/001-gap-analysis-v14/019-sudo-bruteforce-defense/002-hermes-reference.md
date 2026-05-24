# 019 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Command interception | `hermes-agent/tools/terminal.py` argv pre-parse |
| Sudo policy | three modes: `block`, `confirm`, `allow` |
| Allowlist | regex list of permitted commands when mode = `allow` (e.g. `^sudo /usr/bin/apt install [a-z0-9-]+$`) |
| Brute-force counter | `sudo` attempts across the session capped at N (default 1); subsequent attempts return error to LLM (don't even try) |
| `sudo -S` password scan | reject if stdin contains anything that *looks* like a password attempt |

## Behaviour Matrix

| Mode | Behaviour |
|------|-----------|
| `block` (default) | Tool returns `ToolError::Forbidden("sudo blocked")` without invoking shell |
| `confirm` | TUI / gateway prompts the user; on no-response in 60s → block |
| `allow` | Allowlist regex must match; otherwise treated as `confirm` |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
