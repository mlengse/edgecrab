# 018 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| OSC 8 emitter | `hermes-agent/hermes_cli/render/osc8.py` |
| URL pattern | `https?://` matched then wrapped |
| File pattern | `[/\.\w-]+\.(py|rs|ts|md|...):\d+(-\d+)?` |
| Capability gate | reads `TERM_PROGRAM` + `TERM` env to detect support; falls back to plain text on unknown terminals |
| Escape format | `\x1b]8;;URL\x1b\\TEXT\x1b]8;;\x1b\\` |

## File:Line → URL Mapping

| Editor | URL pattern | Trigger env |
|--------|-------------|-------------|
| VS Code | `vscode://file{abs_path}:{line}` | `TERM_PROGRAM=vscode` or user setting |
| Cursor | `cursor://file{abs_path}:{line}` | user setting |
| Sublime | `subl://open?url=file://{abs_path}&line={line}` | user setting |
| Generic | `file://{abs_path}` | fallback |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
