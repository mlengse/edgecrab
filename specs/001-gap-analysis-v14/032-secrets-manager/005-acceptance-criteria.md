# 032 — Acceptance Criteria

## Resolver seam

- [ ] `crates/edgecrab-security/src/secrets.rs` exports `SecretResolver`,
      `SecretBackend`, and a zeroizing `SecretString`.
- [ ] All provider-key reads in `model_router.rs` and gateway adapters go
      through `SecretResolver::resolve` — no direct `std::env::var` for
      credentials remains (grep proves it).

## Backwards compatibility (must not break existing users)

- [ ] With no backend configured, `resolve("ANTHROPIC_API_KEY")` returns
      the same value as `std::env::var` does today.
- [ ] An existing `~/.edgecrab/.env`-only setup runs unchanged with zero
      config edits.

## Vault / keychain backends

- [ ] OS keychain backend stores and retrieves a secret round-trip
      (behind cargo feature, gracefully absent if feature off).
- [ ] Vault backend resolves a key by name using a single
      `secrets.bootstrap_token_env` token; vault values are **never**
      written to `.env` or any plaintext file.
- [ ] Resolver chain order is `vault → keychain → env`, first hit wins,
      configurable via `secrets.backend`.

## Custody guarantees

- [ ] `SecretString` zeroizes on drop and has no `Debug`/`Display` that
      prints the value.
- [ ] A resolved secret accidentally logged is masked by
      [redact.rs](../../../crates/edgecrab-security/src/redact.rs) (regression test).
- [ ] Any on-disk cache file is created `0o600` (mirrors `mcp-tokens`).

## CLI

- [ ] `edgecrab secret set/get/list` work against the active backend.
- [ ] A migration path moves existing `.env` keys into the chosen backend.

## Non-regression

- [ ] `cargo test --workspace` green; `cargo clippy --workspace -- -D warnings` clean.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
