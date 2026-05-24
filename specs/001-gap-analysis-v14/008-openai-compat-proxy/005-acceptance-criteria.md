# 008 — Acceptance Criteria

## Functional

- [ ] `edgecrab proxy --port 11434` starts a localhost server.
- [ ] `curl http://127.0.0.1:11434/v1/models` returns OpenAI-shape JSON
      with the configured aliases.
- [ ] OpenAI Python SDK (`OpenAI(base_url="http://127.0.0.1:11434/v1",
      api_key="<token>")`) successfully runs a non-streaming chat completion
      against an Anthropic backend.
- [ ] Streaming (`stream=True`) produces byte-correct OpenAI SSE format
      indistinguishable from the real OpenAI API.
- [ ] Tool/function calling round-trips: OpenAI `tools` schema in,
      provider-native call out, `tool_calls` back to client.
- [ ] Aider, Cline, and the official OpenAI SDK all work against the proxy
      with no client-side modifications.
- [ ] Final SSE buffer flush at EOF works (regression test from prior
      `input_json_delta` truncation bug).

## Security

- [ ] Requests without a valid Bearer token return 401.
- [ ] Server refuses to start if token unset.
- [ ] `--bind 0.0.0.0` requires `--allow-public` flag.
- [ ] Token redacted in all logs.
- [ ] Test: malformed Authorization header → 401, not 500.

## Error Mapping

- [ ] 429 from backend surfaces as OpenAI 429.
- [ ] Anthropic `overloaded_error` → 503.
- [ ] Invalid backend model → OpenAI 404 model_not_found.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `cargo test -p edgecrab-proxy` ≥ 20 tests.
- [ ] Translator functions are pure (no I/O); covered by table-driven tests.

## Documentation

- [ ] `AGENTS.md` gains a "Proxy" section.
- [ ] `README.md` adds quick-start: `edgecrab proxy` + Aider config snippet.
- [ ] Security model documented (localhost default, public flag).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
