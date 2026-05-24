# 003 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| LSP client (subprocess-managed) | Hermes uses external LSP via `tools/file_operations.py` post-write hook |
| Post-write delta lint | `hermes-agent/tools/file_operations.py` — after a successful write/patch, dispatches a `textDocument/diagnostic` request and waits up to N ms |
| Language detection | extension → server map (`rust → rust-analyzer`, `py → pylsp`, `ts → tsserver`, ...) |
| Diagnostic rendering | Severity-filtered (Error + Warning), de-duplicated, attached as a system note tool-result |

## Behaviour

```
1. Tool: write_file("src/foo.rs", new_content)
2. Tool returns OK with stripped diagnostics:
       {
         "ok": true,
         "path": "src/foo.rs",
         "diagnostics": [
           {"severity":"error","line":42,"msg":"cannot find type `Bar`"},
           ...
         ]
       }
3. Model sees diagnostics in its next tool-result slot and either
   acknowledges or fires another patch_file.
```

The key insight: diagnostics are returned **as part of the tool result**,
not as a separate user message. This keeps them in the model's tool-call
attention pattern (much higher salience than a new user turn).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
