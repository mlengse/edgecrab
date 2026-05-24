# 028 — Acceptance Criteria

## Functional

- [ ] `/skills tap add https://example.com/edgecrab-tap` shows
      publisher key fingerprint; user confirms; key pinned.
- [ ] `/skills install official/git-workflow` fetches from tap,
      verifies hash + signature, installs.
- [ ] Tampered manifest (signature invalid) → install rejected with
      clear message.
- [ ] Tampered skill blob (hash mismatch) → install rejected.
- [ ] Key rotation requires explicit `--rotate-key` (TOFU enforced).
- [ ] `/skills tap update` shows diff (new/updated/removed) before
      applying.
- [ ] Direct GitHub install still works, marked `[unverified]`,
      `skills_guard` scan still runs.

## Security

- [ ] Ed25519 verification uses `ed25519-dalek` (audited).
- [ ] Pinned key stored in `tap config.toml` chmod 0600.
- [ ] Skill files written atomically (.tmp → rename).
- [ ] Provenance comment present in every installed skill.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Tamper-detection tests for both manifest and blob.
- [ ] `SignatureVerifier` trait abstracts the algorithm.

## Documentation

- [ ] Threat model section: what trusted taps protect against and
      what they don't.
- [ ] `AGENTS.md` extended with tap workflow.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
