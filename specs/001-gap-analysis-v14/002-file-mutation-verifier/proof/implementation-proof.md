# 002 — File-Mutation Verifier — Implementation Proof

**Branch:** `feat/file-mutation-verifier`  
**Date:** 2026-05-24  
**Status:** Implemented

## What Was Built

| Component | Location |
|-----------|----------|
| Mutation buffer + renderers | `crates/edgecrab-tools/src/mutations.rs` |
| ToolContext hook | `crates/edgecrab-tools/src/registry.rs` (`mutation_turn`, `record_mutation`) |
| Turn lifecycle + failure tracking | `crates/edgecrab-core/src/conversation.rs` |
| Stream event | `StreamEvent::Footer` in `crates/edgecrab-core/src/agent.rs` |
| TUI display | `crates/edgecrab-cli/src/app.rs` (`AgentResponse::Footer`) |
| Gateway display | `crates/edgecrab-gateway/src/event_processor.rs` |
| Config | `display.file_mutation_verifier` + `EDGECRAB_FILE_MUTATION_VERIFIER` |

### Behaviour

1. **Success log** — After each user turn, if `write_file`, `patch`, or `apply_patch` succeeded, a footer lists paths with `A`/`M`/`D` glyphs and `+lines −lines`.
2. **Failure advisory (Hermes parity)** — Paths where mutation tools failed without a later success in the same turn get a verifier warning (prevents “claimed edit, git status says no”).
3. **Next-turn context** — Footer is injected as a `user` message prefixed `[file-mutation-verifier]` (cache-safe; not system-prompt mutation).
4. **Empty turns** — Read-only turns produce no footer.
5. **Reset** — `MutationTurnState::clear()` at `execute_loop` entry.

## Tests Run

```bash
cargo test -p edgecrab-tools mutations::
cargo test -p edgecrab-core --test file_mutation_verifier
cargo test --workspace   # full suite
cargo clippy --workspace -- -D warnings
```

## Brutal Honest Assessment vs Nous Hermes Agent

| Dimension | Hermes (Python) | EdgeCrab (Rust) | Verdict |
|-----------|-----------------|-----------------|---------|
| **Failure verifier** | `_turn_failed_file_mutations` + footer on over-claim | Same state machine in `MutationTurnState::record_tool_outcome` | **Parity** |
| **Success mutation log** | Not in current Hermes (`files-mutated` in gap spec is aspirational) | Implemented per EdgeCrab spec | **Exceeds Hermes** |
| **Tool coverage** | `write_file`, `patch` | `write_file`, `patch`, `apply_patch` | **Exceeds** |
| **V4A multi-file** | Regex path extraction | Same header parsing in `extract_file_mutation_targets` | **Parity** |
| **Lint/LSP “landed”** | `file_mutation_result_landed` treats write with lint as landed | `ok` + no top-level `error` field | **Mostly parity** (EdgeCrab may need lint-field refinement later) |
| **Checkpoint integration** | Shares buffer with checkpoint manager | Not wired to checkpoint v2 yet | **Gap** (related spec 006) |
| **Performance** | Python dict per turn | `Mutex` + bounded vec (256) | **Rust-appropriate; <5 ms target met in unit scope** |
| **Config** | `display.file_mutation_verifier` + env | Same semantics | **Parity** |

### Summary

EdgeCrab **matches** Hermes on the production-critical failure verifier and **exceeds** it with the success mutation log the gap analysis described (Hermes docs mention verifier for failures only; success ground-truth footer is an EdgeCrab addition). Remaining gaps are checkpoint sharing and optional lint-aware “landed” refinement — acceptable follow-ups, not blockers for this tier-S feature.

## Known Edge Cases & Mitigations

| Edge case | Mitigation |
|-----------|------------|
| Parallel tool batch | Shared `Arc<MutationTurnState>` across parallel `DispatchContext` clones |
| >256 files/turn | Collapse with `… + N more` |
| Schema-resolution `ToolContext` | `mutation_turn: None` — no false records |
| Sub-agent / reflection tools | Isolated empty `MutationTurnState` — does not pollute parent footer |
| Termux width <60 | `render_success_footer_width` compact header (available for TUI wiring) |
| Unicode paths | `truncate_path` uses char counts, not byte slices |
