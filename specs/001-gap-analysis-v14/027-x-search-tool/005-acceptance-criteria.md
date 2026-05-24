# 027 — Acceptance Criteria

## Functional

- [ ] `x_search(query="edgecrab")` returns list of recent tweets with
      id, author, text, created_at, metrics, url.
- [ ] Missing `X_BEARER_TOKEN` → clear error.
- [ ] 429 response → backoff + retry (up to 3); final failure surfaces
      `Retry-After`.
- [ ] `max_results` clamped to [10, 100] (per X v2 limits).
- [ ] SSRF guard prevents accidental private-IP target.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `SocialSearchBackend` trait designed even though only one impl.
- [ ] Wiremock integration tests covering success + 429 + 500.

## Documentation

- [ ] `AGENTS.md` tools table updated.
- [ ] Cost note: "X v2 has paid tiers; track usage carefully."

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
