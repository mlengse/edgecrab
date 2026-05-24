# 030 — Acceptance Criteria

## Functional

- [ ] A plugin registering an `OutputTransformer` with `modes=EndOfTurn`
      transforms the final assistant message before display.
- [ ] A `Delta`-mode plugin transforms each streamed token in order.
- [ ] Built-in redaction still applies (refactored into pipeline; no
      regression).
- [ ] Built-in OSC8 (folder 018) participates in the pipeline.
- [ ] UTF-8 multi-byte tokens split across stream chunks are buffered
      and reassembled before passing to transformers.
- [ ] Slow transformer (> 1 ms timeout) falls through with original
      token; warn logged.
- [ ] `--no-plugins` flag bypasses user plugins; built-ins still run.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Pipeline tests with N stub transformers ensure ordering.
- [ ] UTF-8 boundary test ensures no panics on mid-codepoint split.

## Documentation

- [ ] `AGENTS.md` plugin section adds `OutputTransformer`.
- [ ] Sample plugin: inline citation linker shipped under `examples/`.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
