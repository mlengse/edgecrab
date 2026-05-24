# 028 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-tools/src/tools/skills_hub/ (refactor)            │
   │                                                                  │
   │   mod.rs                                                         │
   │   tap.rs              Tap config + add/remove                    │
   │   manifest.rs         Manifest JSON schema + parser              │
   │   verify.rs           Ed25519 signature verification             │
   │   keystore.rs         Publisher key pinning (TOFU)               │
   │   install.rs          Tap-aware install: lookup → verify → save  │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       Storage layout                                             │
   │                                                                  │
   │   ~/.edgecrab/taps/                                              │
   │     official/                                                    │
   │       config.toml      (name, url, key_id, pinned_pubkey_hex)    │
   │       manifest.json    (cached, signature-verified)              │
   │       skills/<id>.md   (verified blobs)                          │
   │     mycompany/                                                   │
   │     community/                                                   │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Refactor** | `crates/edgecrab-tools/src/tools/skills_hub.rs` → `skills_hub/` module |
| **Manifest format** | `tap.json` containing `{"version":1,"publisher":"name","skills":[{"id":"…","sha256":"…","version":"…","description":"…"}],"signature":"base64(Ed25519(skills_canonical))"}` |
| **Sig algo** | Ed25519 via `ed25519-dalek` crate (audited, well-maintained) |
| **CLI** | `/skills tap add <url> [--key=<hex>]`, `/skills tap list`, `/skills tap remove <name>`, `/skills tap update`, `/skills install <tap>/<skill_id>` |
| **TOFU** | first `tap add` without `--key` shows fingerprint, asks user to confirm; subsequent updates verify the same key; rotation requires explicit `--rotate-key` |
| **Backward-compat** | direct GitHub install (existing) still works but emits `[unverified]` warning and runs full `skills_guard` scan |
| **Manifest refresh** | `/skills tap update` pulls latest manifest, verifies signature, compares to cache, diffs new/updated/removed |
| **Skill install from tap** | hash verified before write to `~/.edgecrab/skills/` |
| **Tests** | generate test keypair; sign test manifest; verify; tamper-detection test (modify one skill bytes → verify fails) |

## Risks

- Key management UX is hard. Always show fingerprints; never auto-rotate.
- A compromised tap is catastrophic. Document threat model clearly:
  trusted taps are no stronger than the publisher's key hygiene.
- Mixing trusted + untrusted installs in one skills dir → mark
  source provenance in skill files (`<!-- source: tap=official id=…
  sha256=… -->`).

## DRY / SOLID Notes

- **SRP:** manifest parse / sig verify / keystore / install are
  separate.
- **OCP:** different signing schemes (minisign, sigstore) slot in
  behind a `SignatureVerifier` trait.
- **DRY:** existing `skills_guard.rs` scan still runs as a *second*
  defence layer on every install.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
