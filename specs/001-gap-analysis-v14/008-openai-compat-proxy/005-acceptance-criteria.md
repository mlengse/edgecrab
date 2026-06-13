# 008 — Acceptance Criteria

## Functional

- [x] `edgecrab proxy start --port 11434` starts a localhost server.
- [x] `curl http://127.0.0.1:11434/v1/models` returns OpenAI-shape JSON
      with the configured aliases (e2e + unit).
- [ ] OpenAI Python SDK against live Anthropic backend (manual / CI with keys).
- [x] Streaming (`stream=True`) produces OpenAI SSE with `[DONE]` (e2e mock).
- [x] OpenAI `tools` schema accepted; wire mapping unit-tested; mock e2e 200.
- [ ] Aider, Cline, and the official OpenAI SDK — manual smoke (same wire shape).
- [x] Final SSE EOF flush without `Finished` (unit `sse_eof_flush_*`).

## Security

- [x] Requests without a valid Bearer token return 401.
- [x] Server auto-generates token when missing on localhost start (`ensure_proxy_token`).
- [x] Non-loopback bind requires `--allow-public` + token file (unit).
- [x] Token redacted in logs (no token in tracing fields).
- [x] Malformed Authorization header → 401 (e2e).

## Error Mapping

- [x] `LlmError::RateLimited` → OpenAI 429 (unit).
- [x] `overloaded` in Api/ProviderError → 503 (unit).
- [x] Unknown / invalid model → 404 `model_not_found` (e2e + unit).

## Code Quality

- [x] `cargo clippy -p edgecrab-proxy -- -D warnings`.
- [x] `cargo test -p edgecrab-proxy` — incl. Nous quarantine e2e + grok/xAI OAuth + CLI/slash e2e.
- [x] Nous Hermes parity: inference URL allowlist, terminal refresh quarantine, auth.json flock, start preflight.
- [x] Wire translators pure in `wire/messages.rs`, `wire/sse.rs` (no I/O).

## Documentation

- [x] `AGENTS.md` — OpenAI-Compatible Proxy section.
- [x] `README.md` — quick-start + Aider snippet.
- [x] Security model (localhost default, `--allow-public`) in README + plan.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
