# 025 — Acceptance Criteria

## Functional

- [ ] `EDGECRAB_LANG=fr edgecrab` → system prompt identity is French.
- [ ] `/lang ja` → setup wizard + errors switch to Japanese.
- [ ] French user prompt "résume ce fichier" tends to get a French
      response (system prompt + localised identity nudges output
      language; verify on at least 3 popular models).
- [ ] Tool *schemas* remain English (LLM doesn't see translated
      descriptions).
- [ ] Unknown lang code → fall back to `en` with warning.

## CI

- [ ] CI check: every key in `en.toml` exists in every other locale
      file. Missing keys → CI fails.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `t!()` macro pattern is uniform; no inline English remains in
      identified Phase-1 sites.

## Documentation

- [ ] `AGENTS.md` adds `config.lang` + `EDGECRAB_LANG` env.
- [ ] Contribution guide explains translation workflow.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
