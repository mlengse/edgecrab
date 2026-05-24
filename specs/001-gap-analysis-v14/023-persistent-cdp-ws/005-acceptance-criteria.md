# 023 — Acceptance Criteria

## Functional / Performance

- [ ] Second call to `browser_console` on same page < 20 ms (vs.
      ~300 ms baseline).
- [ ] Navigate → screenshot → click → scrape sequence < 300 ms total
      CDP overhead.
- [ ] After WS crash, next browser tool call reconnects and succeeds.
- [ ] On Agent drop, Chrome subprocess is killed (verified by
      `pgrep`).
- [ ] Profile dir retention setting honoured.

## Cross-Platform

- [ ] macOS: pool works.
- [ ] Linux (X11 + Wayland): pool works.
- [ ] Windows: pool works AND parent exit kills children.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Mock `BrowserClient` used in tool unit tests.
- [ ] No leaked Chrome processes after `cargo test`.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
