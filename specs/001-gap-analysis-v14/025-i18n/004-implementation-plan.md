# 025 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/i18n/ (NEW module)                       │
   │                                                                  │
   │   mod.rs              language selection + global accessor       │
   │   catalog.rs          Catalog struct: HashMap<key, String>       │
   │   locales/                                                       │
   │     en.toml                                                      │
   │     fr.toml                                                      │
   │     de.toml                                                      │
   │     ... (16 total)                                               │
   │                                                                  │
   │   pub fn set_lang(lang: &str)                                    │
   │   pub fn t(key: &str) -> &'static str                            │
   │   pub fn tf(key: &str, args: &[(&str,&str)]) -> String           │
   │                                                                  │
   │   Catalogs embedded via include_str!; lazy-parsed once.          │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       Call sites                                                 │
   │                                                                  │
   │   prompt_builder.rs:                                             │
   │     - DEFAULT_IDENTITY  → t("identity.default")                  │
   │     - MEMORY_GUIDANCE   → t("memory.guidance")                   │
   │     - SKILLS_GUIDANCE   → t("skills.guidance")                   │
   │                                                                  │
   │   commands.rs: slash help text → t("commands.help.{name}")       │
   │                                                                  │
   │   ToolError → Display impl looks up t("errors.<variant>")        │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New crate module** | `crates/edgecrab-core/src/i18n/` |
| **Locales** | `crates/edgecrab-core/src/i18n/locales/*.toml` (16 files) |
| **`t!` macro** | exported from `edgecrab-core`; consumers use `t!("identity.default")` |
| **Language selection** | `EDGECRAB_LANG` env → `config.lang` → system locale → fallback `en` |
| **System locale detection** | use `sys-locale` crate |
| **Slash command** | `/lang <code>` to switch at runtime; persists to config |
| **Phased extraction** | Phase 1: system prompt + setup wizard + top 20 errors (English baseline); Phase 2: slash help; Phase 3: tool descriptions |
| **Translation workflow** | seed translations from a known-good translator; mark machine-translated entries `# auto`; humans review later |
| **Tests** | every locale must parse; every key in `en.toml` must exist in every other locale (CI check) |

## Risks

- Translation drift: keys added in `en.toml` not in others. CI gate
  fails on missing keys.
- Bundle size: 16 TOMLs of ~200 keys each ≈ 200 KB. Acceptable.
- Tool *descriptions* sent to LLM — translating them might confuse the
  LLM with mixed-language schemas. Keep tool schemas English (LLMs
  trained largely on English), only translate user-facing text.

## DRY / SOLID Notes

- **SRP:** catalog is data; lookup is logic; selection is policy.
- **OCP:** add a locale = one file.
- **DRY:** all strings flow through `t!` — never duplicated.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
