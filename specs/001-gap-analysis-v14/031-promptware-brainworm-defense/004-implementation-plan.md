# 031 — Implementation Plan

## Strategy

Consolidate first (DRY), then add the two missing chokepoints. Do **not**
write new pattern lists — collapse the four existing ones into a single
source in `edgecrab-security`, then call it from the two undefended paths.

## Architecture

```
                       ┌───────────────────────────────────────┐
                       │  edgecrab-security::threat_patterns     │
                       │  (SINGLE SOURCE OF TRUTH)               │
                       │  - INJECTION_PATTERNS                    │
                       │  - EXFIL_PATTERNS                        │
                       │  - BRAINWORM_PATTERNS (new, ~15)         │
                       │  fn scan(text, ScanContext) -> Verdict   │
                       └───────────────────────────────────────┘
                          ▲          ▲          ▲          ▲
            ┌─────────────┘   ┌──────┘    ┌─────┘    ┌─────┘
            │                 │           │          │
   ┌────────┴───────┐ ┌───────┴──────┐ ┌──┴───────┐ ┌┴──────────────┐
   │ memory WRITE   │ │ memory LOAD  │ │ context  │ │ plugin/skill  │
   │ memory.rs:266  │ │ prompt_      │ │ files    │ │ guard.rs      │
   │ (exists)       │ │ builder (NEW)│ │ (exists) │ │ (migrate)     │
   └────────────────┘ └──────────────┘ └──────────┘ └───────────────┘

   ┌──────────────────────────────────────────────────────────────┐
   │ conversation.rs ReAct loop                                    │
   │   result = registry.dispatch(...)                             │
   │   wrapped = wrap_tool_result(call.id, result)   ← NEW         │
   │     ┌──────────────────────────────────────────────────┐     │
   │     │ ⟦EDGECRAB:TOOL_RESULT id=abc⟧                     │     │
   │     │ <verbatim, never trusted as instructions>         │     │
   │     │ ⟦/EDGECRAB:TOOL_RESULT⟧                           │     │
   │     └──────────────────────────────────────────────────┘     │
   │   messages.push(wrapped)                                      │
   └──────────────────────────────────────────────────────────────┘
```

## File Map

| File | Change |
|------|--------|
| [crates/edgecrab-security/src/threat_patterns.rs](../../../crates/edgecrab-security/src/) | **NEW** — unify `INJECTION_PATTERNS` + plugin `THREAT_PATTERNS` + skill patterns; add Brainworm/C2 set; expose `scan(text, ScanContext) -> Verdict` |
| [crates/edgecrab-security/src/injection.rs](../../../crates/edgecrab-security/src/injection.rs) | Re-export from `threat_patterns`; keep `check_memory_content` as thin wrapper |
| [crates/edgecrab-security/src/lib.rs](../../../crates/edgecrab-security/src/lib.rs) | `pub mod threat_patterns;` |
| [crates/edgecrab-plugins/src/guard.rs](../../../crates/edgecrab-plugins/src/guard.rs#L79) | Delete local `THREAT_PATTERNS`; consume shared module |
| [crates/edgecrab-core/src/prompt_builder.rs](../../../crates/edgecrab-core/src/prompt_builder.rs#L772) | `scan_for_injection` delegates to shared module; **add load-time scan** of recalled `MEMORY.md`/`USER.md` |
| [crates/edgecrab-core/src/conversation.rs](../../../crates/edgecrab-core/src/conversation.rs) | Add `wrap_tool_result()` — delimiter markers around every tool result before `messages.push` |
| [crates/edgecrab-core/src/config.rs](../../../crates/edgecrab-core/src/config.rs#L1481) | Extend existing `injection_scanning` flag → `security.tool_output_delimiters`, `security.scan_recalled_memory` (default `true`) |

## DRY / SOLID Notes

- **DRY:** one threat list, four consumers. Adding a pattern protects all
  chokepoints at once — the whole point of the Hermes consolidation.
- **SRP:** `edgecrab-security` owns *what is dangerous*; each call site
  owns *what to do about it* (block install vs. delimit vs. drop entry).
- **OCP:** new attack classes = append to `BRAINWORM_PATTERNS`, no call
  site changes.
- **Reuse, don't fork:** the delimiter format mirrors the existing
  `ENTRY_DELIMITER` convention already used in memory/migrate crates.

## Sequencing

1. Create `threat_patterns.rs`, migrate the four lists, add Brainworm set.
2. Point `injection.rs`, `guard.rs`, `prompt_builder.rs` at it (no behaviour change — pure refactor, tests stay green).
3. Add load-time memory scan in `prompt_builder`.
4. Add `wrap_tool_result()` in `conversation.rs`.
5. Config flags + docs.

## Cross-References

- [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
