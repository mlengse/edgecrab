# 028 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Tap manifest | JSON document at `https://{tap}/manifest.json` containing list of skills with sha256 and Ed25519 signature |
| Publisher key | Ed25519 public key embedded in tap config; user accepts on first `hermes skills tap add` |
| Verification | each skill download verified against manifest sha256 and signature against publisher key |
| Trust on First Use (TOFU) | publisher key pinned; rotation requires user confirmation |
| `~/.hermes/taps/<tap>/` | cached manifest + verified skill blobs |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
