# 032 — Hermes Reference

## Source

Hermes v0.15.0 — "Bitwarden Secrets Manager: one bootstrap token replaces
per-provider API keys" — PR
[#33402](https://github.com/NousResearch/hermes-agent/pull/33402).

## What Hermes Did

| Aspect | Hermes behaviour |
|--------|------------------|
| Bootstrap | A single `BWS_ACCESS_TOKEN` is the only secret on the host |
| Resolution | Provider keys are fetched from Bitwarden Secrets Manager at runtime by secret name |
| Fallback | If the vault is unconfigured, the agent falls back to environment variables (backwards compatible) |
| At rest | Individual provider keys need not be written to disk at all |
| Rotation | Rotate centrally in Bitwarden; every host picks up the new value on next fetch |

## Threat Model It Closes

- A leaked host filesystem no longer leaks every provider key — only a
  revocable bootstrap token (which can be scoped + expired).
- A malicious skill that reads `~/.hermes/.env` finds nothing useful.
- Key rotation is O(1) central, not O(hosts × providers) manual edits.

## Design Principle

The vault is a **pluggable backend behind a resolver**, not a hard
dependency. No token configured ⇒ behave exactly as before. This is the
contract EdgeCrab should copy so existing `.env` users are unaffected.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
