# 032 — EdgeCrab Current State (Code Is Law)

## How secrets are handled today

| Stage | Code path |
|-------|-----------|
| Storage | `~/.edgecrab/.env` — plaintext flat file |
| Write | [crates/edgecrab-cli/src/setup.rs](../../../crates/edgecrab-cli/src/setup.rs#L1192) "Save an API key to the ~/.edgecrab/.env file" |
| Gateway secrets | [crates/edgecrab-cli/src/gateway_setup.rs](../../../crates/edgecrab-cli/src/gateway_setup.rs#L1860) `.env` file management, `read_env_key()` (L1942) |
| Load | [crates/edgecrab-core/Cargo.toml](../../../crates/edgecrab-core/Cargo.toml#L33) `dotenvy` → process env at startup |
| Read at runtime | `std::env::var("ANTHROPIC_API_KEY")` etc. throughout providers |

## Code is law: there is no vault backend

A workspace-wide search for `keyring`, `keychain`, `vault`, and
`bitwarden` returns **zero** matches. The only secret-handling crate code
is:

- [crates/edgecrab-security/src/redact.rs](../../../crates/edgecrab-security/src/redact.rs) — masks `sk-…` / `api_key` in logs and output (defence *after* the fact).
- [crates/edgecrab-security/src/injection.rs](../../../crates/edgecrab-security/src/injection.rs#L92) — flags `~/.edgecrab/.env` as an exfil target in memory/skill content.

Both treat the `.env` file as the crown-jewel asset to protect — which
confirms it is the single point of failure. There is one bright spot:
MCP Bearer tokens are stored separately at `chmod 0o600`
(`~/.edgecrab/mcp-tokens/`), so a per-secret file-permission pattern
already exists to generalise from.

## What's missing vs. Hermes

1. No single bootstrap secret — N long-lived keys on every host.
2. No `SecretResolver` indirection — call sites hit `std::env::var`
   directly, so there is no seam to insert a vault behind.
3. No memory-only secret lifetime — keys are on disk for the process
   lifetime and beyond.

## What to reuse

- The `mcp-tokens` `0o600` pattern for any local cache.
- `redact.rs` already keeps secrets out of output — no new redaction
  needed, just route resolved secrets through it.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
