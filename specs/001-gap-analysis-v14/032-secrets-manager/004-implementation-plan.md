# 032 — Implementation Plan

## Strategy

Introduce a `SecretResolver` seam in `edgecrab-security`, route all
provider-key reads through it, then add backends behind it. The first
backend is the OS keychain (zero external infra); a vault HTTP backend
follows the same trait. `.env` stays as the default fallback so nothing
breaks.

## Architecture

```
   provider construction (model_router.rs, gateway adapters)
            │  needs "ANTHROPIC_API_KEY"
            ▼
   ┌─────────────────────────────────────────────┐
   │ edgecrab-security::secrets::SecretResolver    │
   │   fn resolve(name) -> Option<SecretString>    │
   │   (ordered chain, first hit wins)             │
   └─────────────────────────────────────────────┘
        │            │              │
        ▼            ▼              ▼
   ┌─────────┐  ┌──────────┐  ┌──────────────┐
   │ Vault    │  │ OS       │  │ Env / .env   │
   │ backend  │  │ keychain │  │ (dotenvy)    │
   │ (HTTP,   │  │ backend  │  │ DEFAULT      │
   │  bootstrap│  │          │  │ FALLBACK     │
   │  token)  │  │          │  │              │
   └─────────┘  └──────────┘  └──────────────┘
        │
        ▼
   SecretString  ─── never written to .env, zeroized on drop,
                     routed through redact.rs before any log/output
```

## File Map

| File | Change |
|------|--------|
| [crates/edgecrab-security/src/secrets.rs](../../../crates/edgecrab-security/src/) | **NEW** — `SecretResolver`, `SecretBackend` trait, `SecretString` (zeroizing) |
| [crates/edgecrab-security/src/lib.rs](../../../crates/edgecrab-security/src/lib.rs) | `pub mod secrets;` |
| [crates/edgecrab-security/Cargo.toml](../../../crates/edgecrab-security/Cargo.toml) | add `secrecy`/`zeroize`; optional `keyring` feature |
| [crates/edgecrab-core/src/model_router.rs](../../../crates/edgecrab-core/src/model_router.rs) | resolve provider keys via `SecretResolver`, not raw `std::env::var` |
| [crates/edgecrab-cli/src/gateway_setup.rs](../../../crates/edgecrab-cli/src/gateway_setup.rs#L1942) | `read_env_key` delegates to resolver |
| [crates/edgecrab-core/src/config.rs](../../../crates/edgecrab-core/src/config.rs) | `secrets.backend` (`env`\|`keychain`\|`vault`), `secrets.bootstrap_token_env`, `secrets.vault_url` |
| [crates/edgecrab-cli/src/cli_args.rs](../../../crates/edgecrab-cli/src/cli_args.rs) | `edgecrab secret set/get/list` subcommands |

## DRY / SOLID Notes

- **DSP (Dependency Inversion):** call sites depend on the
  `SecretResolver` abstraction, not on `dotenvy` or any specific vault.
- **OCP:** new backend = new `impl SecretBackend`, no call-site edits.
- **Backwards-compatible:** unconfigured ⇒ resolver chain ends at the
  env backend ⇒ identical behaviour to today.
- **Reuse:** route every resolved secret through existing
  [redact.rs](../../../crates/edgecrab-security/src/redact.rs); reuse the
  `mcp-tokens` `0o600` file pattern for any on-disk cache.

## Sequencing

1. `secrets.rs` with `SecretResolver` + env backend (pure refactor of
   existing `std::env::var` reads — behaviour unchanged, tests green).
2. OS keychain backend behind a cargo feature.
3. Vault HTTP backend + bootstrap token config.
4. `edgecrab secret` CLI + migration helper from `.env`.

## Cross-References

- [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
