# 017 — Acceptance Criteria

## Functional

- [ ] `terminal` error containing `sk-abc...XYZ` is reduced to `sk-<REDACTED>`
      before reaching the LLM context.
- [ ] `/Users/alice/.ssh/id_rsa` → `/Users/<USER>/.ssh/id_rsa`.
- [ ] AWS access key, GitHub PAT, Slack token, JWT, Bearer header, and
      private-IP patterns all redacted.
- [ ] `file_search` (KeepPaths policy) does NOT redact `/Users/<USER>/`
      because line refs help the agent.
- [ ] Final assistant output uses the SAME catalog (regression for
      consistency).

## Performance

- [ ] `sanitize_text` on a 4 KB input completes in < 50 µs (benchmark).
- [ ] No measurable regression in tool-error round-trip latency.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 30 pattern test cases (positive + negative per pattern).
- [ ] Public API `#[non_exhaustive]` on `RedactionPolicy`.

## Documentation

- [ ] `AGENTS.md` Security Model table updated; remove "tool errors not
      sanitised" gap.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
