# 014 — Acceptance Criteria

## Functional

- [ ] `web_search` with `primary: "searxng"` succeeds against a public
      SearXNG instance.
- [ ] Setting Brave key and `primary: "brave"` works.
- [ ] DDGS works with no key.
- [ ] Fallback: configure primary that always 429s; chain falls back to
      next backend and returns results.
- [ ] All-fail: returns descriptive `SearchError` listing tried backends.
- [ ] Plugin can register a new backend at runtime (depends on 009).

## Security

- [ ] SearXNG endpoint URL validated via SSRF guard.
- [ ] API keys never appear in logs (verify with redaction test).

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 12 tests across backends + chain.
- [ ] Backends gated behind cargo features (`searxng`, `brave`, `ddgs`)
      so users can opt out of unused deps.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
