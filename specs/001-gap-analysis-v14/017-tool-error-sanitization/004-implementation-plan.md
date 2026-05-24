# 017 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-security/src/redaction/                           │
   │                                                                  │
   │   mod.rs                                                         │
   │   patterns.rs       (compiled regexes — single lazy_static set)  │
   │   sanitizer.rs      (apply patterns; return cleaned String)      │
   │                                                                  │
   │   pub fn sanitize_tool_error(s: &str, tool: &str) -> String      │
   │   pub fn sanitize_text(s: &str) -> String                        │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/conversation.rs (hook point)             │
   │                                                                  │
   │   match registry.dispatch(call).await {                          │
   │       Ok(result) => push_tool_message(call.id, result),          │
   │       Err(e) => {                                                │
   │           let clean = sanitize_tool_error(&e.to_string(),        │
   │                                            &call.name);          │
   │           push_tool_error_message(call.id, clean);               │
   │       }                                                          │
   │   }                                                              │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-security/src/redaction/mod.rs` |
| **Patterns** | `crates/edgecrab-security/src/redaction/patterns.rs` — `OnceLock<Vec<(Regex, &'static str)>>` |
| **Sanitiser** | `crates/edgecrab-security/src/redaction/sanitizer.rs` — `fn sanitize_text(input: &str) -> String` applies all patterns; benchmarked for hot-path |
| **Per-tool opt-out** | a `ToolHandler::redaction_policy(&self) -> RedactionPolicy` trait method (default: `Full`); options `Full`, `KeepPaths`, `KeepIps`, `None` |
| **Hook** | `crates/edgecrab-core/src/conversation.rs` — sanitise on Err branch |
| **Also apply to Ok results** | gated by config `security.sanitize_tool_output: true` (off by default — would break tools that legitimately emit paths) |
| **DRY with assistant redaction** | refactor existing output redaction to use the same `redaction::sanitize_text` so we have one regex catalog |
| **Tests** | golden-file tests; every pattern has at least one positive + negative case |

## Performance Notes

Run patterns in a single pass using `RegexSet` to test which patterns
match, then `Regex::replace_all` only for matched ones. For typical
short error strings (< 4 KB) this is sub-microsecond and free.

## Per-Tool Policy

| Tool | Policy |
|------|--------|
| `terminal` | Full |
| `file_read` | Full |
| `file_write` | Full |
| `file_search` | KeepPaths (line refs harmless and helpful) |
| `web_search` | KeepIps (URL hosts must remain readable) |
| `vision` | Full |

## DRY / SOLID Notes

- **DRY:** one regex catalog used by tool-error redaction and
  assistant-output redaction. Two callers, one source of truth.
- **SRP:** patterns are data; sanitiser is logic; integration is at the
  conversation loop.
- **OCP:** new pattern = one line in `patterns.rs`.
- **ISP:** `RedactionPolicy` is a small enum; tools default to `Full`.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- DRY's with existing redaction pipeline; both share the same module.
