# 025 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Catalog | `hermes-agent/i18n/{locale}.toml` (en, fr, de, es, pt, it, ja, ko, zh-CN, zh-TW, ar, ru, hi, tr, nl, pl) |
| Loader | `hermes-agent/i18n/loader.py` reads `HERMES_LANG` env or `config.lang`, falls back to en |
| System prompt | identity + memory guidance + platform hints all localised |
| UI strings | slash command help, error messages, setup wizard |
| RTL support | Arabic catalog includes RTL marks where needed |

## Key Catalog

```toml
[identity]
default = "You are EdgeCrab, …"

[memory_guidance]
text = "…"

[errors]
permission_denied = "..."
network_unreachable = "..."

[commands.help]
help = "Show help"
quit = "Quit"
# ... per command
```

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
