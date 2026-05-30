# 031 — Acceptance Criteria

## Single source of truth (DRY)

- [ ] `crates/edgecrab-security/src/threat_patterns.rs` exists and is the
      only place threat needles/severities are declared.
- [ ] `guard.rs`, `injection.rs`, `prompt_builder.rs`, `skills_guard.rs`
      contain **zero** literal threat-pattern lists — all delegate.
- [ ] Adding one pattern to `threat_patterns.rs` is provably exercised by
      a test at every chokepoint (memory write, memory load, context
      file, plugin install, tool output).

## Tool-output delimiters

- [ ] Every tool result pushed in `conversation.rs` is wrapped in
      `⟦EDGECRAB:TOOL_RESULT id=…⟧ … ⟦/EDGECRAB:TOOL_RESULT⟧`.
- [ ] A tool result whose body contains a forged `</tool_result>` or
      `System:` line does **not** break out of the delimited block (the
      forged markers appear as literal content).
- [ ] Delimiter wrapping is gated by `security.tool_output_delimiters`
      (default `true`) and round-trips through compression unchanged.

## Recalled-memory load scan

- [ ] On session start and each re-injection, `MEMORY.md` / `USER.md`
      are scanned by the shared module before entering the prompt.
- [ ] A poisoned memory entry (e.g. `ignore previous instructions`) seeded
      out-of-band is dropped/quarantined at load with a `tracing::warn!`,
      not injected.
- [ ] Gated by `security.scan_recalled_memory` (default `true`).

## Brainworm pattern set

- [ ] ≥ 15 Brainworm/C2/promptware patterns present (forged framing,
      tool-impersonation, exfil-to-webhook, self-replication into memory).
- [ ] Each has a unit test asserting `Verdict::Block` or `::Quarantine`.

## Non-regression

- [ ] Existing tests `injection_scanner_catches_*` and
      `memory_injection_blocked` still pass unchanged.
- [ ] `cargo test --workspace` green; `cargo clippy --workspace -- -D warnings` clean.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
