# 017 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Sanitiser | `hermes-agent/agent/tool_error_sanitizer.py` |
| Patterns | API key regex (`sk-…`, `AKIA…`, etc.), abs path → `<HOME>/…`, IPv4 private ranges → `<PRIVATE_IP>`, JWT → `<JWT>` |
| Hook point | called inside `managed_tool_gateway.py` after the tool returns Err, before the message is pushed back into the loop |
| Whitelisted tools | a few tools (e.g. `file_search` with line numbers) can opt out of certain redactions where they harm correctness |

## Redaction Targets

| Pattern | Replacement |
|---------|-------------|
| `sk-[A-Za-z0-9]{20,}` | `sk-<REDACTED>` |
| `AKIA[0-9A-Z]{16}` | `AKIA<REDACTED>` |
| `xox[bpars]-[0-9A-Za-z-]+` | `xox<REDACTED>` (Slack) |
| `ghp_[A-Za-z0-9]{36}` | `ghp_<REDACTED>` (GitHub) |
| `Bearer\s+[A-Za-z0-9._\-]{20,}` | `Bearer <REDACTED>` |
| `/Users/[^/]+` | `/Users/<USER>` |
| `/home/[^/]+` | `/home/<USER>` |
| `\b(10|192\.168|172\.(1[6-9]|2[0-9]|3[01]))\.\d+\.\d+\b` | `<PRIVATE_IP>` |
| `eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+` | `<JWT>` |
| Stack-trace abs paths | basename only |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
