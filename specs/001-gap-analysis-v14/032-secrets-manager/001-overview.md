# 032 — External Secrets Manager (Bitwarden-class backend)

**Tier:** A | **Impact:** 4 | **Value-per-Effort:** 3 | **Risk:** 3
**Primitive moved:** Trust in side-effects (secret custody)
**Source:** Hermes v0.15.0 — Bitwarden Secrets Manager integration
([#33402](https://github.com/NousResearch/hermes-agent/pull/33402))

## Why It Matters (First Principles)

An agent needs N provider credentials (Anthropic, OpenAI, xAI, Telegram,
…). The question is **where the secret of record lives**. Two designs:

- **Plaintext-at-rest:** every key sits in a flat file the agent process
  can read — and so can any tool, plugin, or compromised dependency that
  reaches the filesystem. Rotation means hand-editing N entries on every
  host.
- **One bootstrap secret:** a single short-lived token unlocks a remote
  vault; individual keys are fetched into memory on demand and never
  written to disk. Rotation happens once, centrally.

The second design shrinks the blast radius from "N long-lived secrets on
every machine" to "one revocable token." For an agent that runs
untrusted skills and remote MCP servers, that difference is the whole
game.

## The Gap

EdgeCrab is firmly in the plaintext-at-rest camp: every credential lives
in `~/.edgecrab/.env`, loaded via `dotenvy` at startup. There is **no**
keyring, OS keychain, or external vault backend — confirmed by zero
`keyring` / `keychain` / `vault` / `bitwarden` references in the
workspace (see [003-edgecrab-current-state.md](003-edgecrab-current-state.md)).

## What "Good" Looks Like

- One bootstrap secret (`BWS_ACCESS_TOKEN`-equivalent) configured once.
- Provider keys resolved through a `SecretResolver` abstraction: try
  vault → fall back to env → fall back to OS keychain.
- Fetched secrets held in memory only; never persisted to `.env`.
- Existing redaction pipeline ([crates/edgecrab-security/src/redact.rs](../../../crates/edgecrab-security/src/redact.rs))
  keeps them out of logs and model output.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Complements: [../031-promptware-brainworm-defense/](../031-promptware-brainworm-defense/) (the `.env` file is the #1 exfil target the threat patterns guard)
