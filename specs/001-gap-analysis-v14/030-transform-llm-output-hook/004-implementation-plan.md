# 030 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-sdk-core/src/plugin.rs (extend Plugin trait)      │
   │                                                                  │
   │   pub trait OutputTransformer: Send + Sync {                     │
   │       fn name(&self) -> &str;                                    │
   │       fn modes(&self) -> TransformModes;  // Delta | EndOfTurn   │
   │       fn transform_delta(&self, &str, &Ctx) -> Result<String>    │
   │       fn transform_eot(&self, &str, &Ctx) -> Result<String>      │
   │   }                                                              │
   │                                                                  │
   │   default impls return input unchanged unless overridden         │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/output_pipeline.rs (NEW)                 │
   │                                                                  │
   │   pub struct OutputPipeline {                                    │
   │       transformers: Vec<Arc<dyn OutputTransformer>>,             │
   │   }                                                              │
   │                                                                  │
   │   impl OutputPipeline {                                          │
   │       fn apply_delta(&self, s: &str, ctx) -> String              │
   │       fn apply_eot(&self, s: &str, ctx) -> String                │
   │   }                                                              │
   │                                                                  │
   │   Built-ins (registered first):                                  │
   │     1. RedactionTransformer (existing redaction pipeline)        │
   │     2. Osc8Transformer       (folder 018)                        │
   │     3. CitationLinker        (optional)                          │
   │     4. ... user plugins ...                                      │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/agent.rs                                  │
   │                                                                  │
   │   on StreamEvent::Token(tok): pipeline.apply_delta(tok)          │
   │   on turn complete:           pipeline.apply_eot(full_msg)       │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Trait** | `crates/edgecrab-sdk-core/src/plugin.rs` — add `OutputTransformer` trait |
| **Pipeline** | `crates/edgecrab-core/src/output_pipeline.rs` |
| **Hook in agent.rs** | both delta and EOT apply points |
| **Built-in: Redaction** | wrap existing redaction logic as a built-in transformer (refactor; preserves behaviour) |
| **Built-in: OSC8** | folder 018 implemented as a transformer |
| **Plugin registration** | extend native plugin loader (folder 009) to register transformers |
| **Ordering** | config `plugins.output_pipeline_order: [...]` — built-ins always last unless overridden |
| **Performance budget** | streamed-delta transformers must complete in < 1 ms per token; pipeline enforces a timeout, falling through with original token on timeout (log warn) |
| **Tests** | each transformer testable in isolation; pipeline tested with N stub transformers ensuring ordering |

## Risks

- Plugins can corrupt LLM output (e.g. broken markdown). Document
  responsibility; provide a `--no-plugins` flag for debugging.
- Streamed-delta mode is fragile around multi-byte UTF-8 boundaries.
  Pipeline buffers incomplete UTF-8 sequences across deltas before
  passing to transformers.

## DRY / SOLID Notes

- **SRP:** each transformer does one thing.
- **OCP:** new transformer = new struct.
- **ISP:** delta vs EOT modes are separate methods with defaults; small
  plugins implement only what they need.
- **DRY:** redaction + OSC8 become first-class transformers; no
  duplicate "post-process the message" paths in the codebase.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Plugin loader: [../009-pluggable-providers-plugins/](../009-pluggable-providers-plugins/)
- OSC8 transformer: [../018-osc8-clickable-urls/](../018-osc8-clickable-urls/)
- Redaction transformer: [../017-tool-error-sanitization/](../017-tool-error-sanitization/)
