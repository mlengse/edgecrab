# 008 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Proxy entry point | `hermes proxy` CLI subcommand (registered in `hermes-agent/hermes_cli/main.py` → handler module) |
| HTTP server | FastAPI/Starlette app exposing `/v1/chat/completions`, `/v1/models`, `/v1/embeddings` (opt) |
| Provider adapter | `hermes-agent/hermes_cli/providers.py` + `hermes-agent/agent/anthropic_adapter.py` translate OpenAI request → native provider call → OpenAI response |
| OAuth providers | `hermes-agent/hermes_cli/auth.py` + `copilot_auth.py` + xAI/Claude Pro auth modules |
| Streaming SSE | OpenAI SSE format on the wire, even when the underlying provider streams differently |
| Model alias map | `claude-3-5-sonnet-claudepro` → Claude Pro OAuth backend; `gpt-5-chatgptpro` → ChatGPT Pro OAuth backend |

## Wire-Level Behaviour

```
Client (Aider, Cline, OpenAI SDK)
       │
       │ POST /v1/chat/completions
       │ Authorization: Bearer <local-token>
       │ body: { model, messages, tools, stream: true }
       │
       ▼
hermes proxy (localhost:port)
       │  validate local Bearer
       │  resolve model alias → backend provider
       │  translate OpenAI tool schema → provider native schema
       │  open provider session (OAuth token from keychain)
       │
       ▼
Backend (Claude Pro, ChatGPT Pro, xAI Grok, Copilot)
       │
       │ streaming tokens
       │
       ▼
proxy re-emits as OpenAI-format SSE
       │
       ▼
Client receives indistinguishable OpenAI stream
```

## Local Auth

Local Bearer token stored in `~/.hermes/proxy-token` (mode 0600). All
incoming requests must present it (defence against same-machine attacks).

## Tool / Function Calling Translation

The hardest part: OpenAI uses `tools: [{type:"function", function:{name,parameters}}]`
and `tool_calls: [{id, function:{name, arguments}}]`. Provider-native shapes
differ (Anthropic, Gemini). The translation table is a pure function in
the adapter module — see Hermes' `tool_backend_helpers.py` style.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
