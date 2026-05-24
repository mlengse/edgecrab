# 004 — Acceptance Criteria

## Functional

- [ ] After two CLI sessions started ≤ 1 h apart with identical SOUL.md /
      AGENTS.md / skills, the second session's first turn reports
      `cache_read_input_tokens` ≥ 80% of the stable-block size.
- [ ] Mutating SOUL.md between sessions invalidates the cache (proven by
      `cache_read_input_tokens = 0` on next session).
- [ ] OpenAI, Gemini, etc. providers behave unchanged.
- [ ] `/cost` displays `cached`, `cache-write`, `input`, `output` columns.

## Cache-Stability Test

- [ ] `cargo test -p edgecrab-core prompt_builder::stable_hash` proves
      the stable hash is invariant under: now(), session_id, cwd,
      and per-turn message history.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `SystemPromptBlocks` is `#[non_exhaustive]` to allow future
      multi-breakpoint expansion without breaking adapters.

## Documentation

- [ ] `AGENTS.md` updates the "Prompt caching policy" section with the
      stable/volatile split rule.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
