# 029 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Model router | `crates/edgecrab-core/src/model_router.rs` (provider factory only — no routing logic) |
| Cost tracking | `crates/edgecrab-core/src/pricing.rs` |
| `/model` command | manual switch |

## What Is Missing

1. Request classifier.
2. Per-tier model selection.
3. Auto-routing on/off config.
4. Stats / savings reporting.
5. Per-turn `@<model>` override.

## Honest Assessment

The mechanics are easy; the *judgement* (which signals → which tier)
needs tuning per user. Ship conservative defaults; allow disable.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
