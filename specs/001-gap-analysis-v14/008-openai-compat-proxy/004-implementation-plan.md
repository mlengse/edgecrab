# 008 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │                       edgecrab-proxy (new crate)                 │
   │                                                                  │
   │   ┌─────────────────┐    ┌────────────────────────────────────┐  │
   │   │ ProxyServer     │    │ OpenAiTranslator (pure functions)  │  │
   │   │ (axum app)      │    │  request_openai_to_native(req,     │  │
   │   │ /v1/chat/...    │───►│        backend) -> NativeChatReq   │  │
   │   │ /v1/models      │    │  response_native_to_openai(resp,   │  │
   │   │ /v1/embeddings  │    │        backend) -> OpenAiChunk     │  │
   │   └────────┬────────┘    └────────────────────────────────────┘  │
   │            │                                                     │
   │            ▼                                                     │
   │   ┌─────────────────────────────────────────────────┐            │
   │   │ AuthMiddleware (Bearer check vs                 │            │
   │   │   ~/.edgecrab/proxy-token)                      │            │
   │   └─────────────────────────────────────────────────┘            │
   │            │                                                     │
   │            ▼                                                     │
   │   ┌─────────────────────────────────────────────────┐            │
   │   │ BackendResolver                                 │            │
   │   │   model_alias  →  Arc<dyn LLMProvider>          │            │
   │   │   (reuses edgecrab-core model_router)           │            │
   │   └─────────────────────────────────────────────────┘            │
   └──────────────────────────────────────────────────────────────────┘
                                │
                                ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │      edgecrab-core providers (existing + OAuth additions)        │
   │                                                                  │
   │   AnthropicApiProvider | ClaudeProOAuthProvider | OpenAiProvider │
   │   ChatGptProOAuthProvider | XaiGrokOAuthProvider | CopilotProv.. │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New crate** | `crates/edgecrab-proxy/` — keeps proxy out of `edgecrab-core` (DIP). Depends on `edgecrab-core`, `edgecrab-types`, `edgecrab-security`, `axum`, `tokio`, `serde_json`. |
| **HTTP server** | `crates/edgecrab-proxy/src/server.rs` — axum router |
| **Endpoints** | `crates/edgecrab-proxy/src/routes/chat.rs`, `routes/models.rs`, `routes/embeddings.rs` (stub initially) |
| **Translator** | `crates/edgecrab-proxy/src/translate/mod.rs` — pure functions per provider backend |
| **Translator per backend** | `translate/anthropic.rs`, `translate/openai.rs`, `translate/gemini.rs`, `translate/grok.rs` |
| **Auth middleware** | `crates/edgecrab-proxy/src/auth.rs` — bearer-token check; token at `~/.edgecrab/proxy-token` (chmod 0600) |
| **Token CLI** | `edgecrab proxy token` subcommands (`set`, `rotate`, `show`) |
| **CLI subcommand** | `crates/edgecrab-cli/src/main.rs` — `edgecrab proxy [--port N] [--bind 127.0.0.1] [--allow-public]` |
| **Config** | `proxy.bind: 127.0.0.1`, `proxy.port: 11434`, `proxy.token_path: ~/.edgecrab/proxy-token`, `proxy.model_aliases: {…}` |
| **Tests** | full e2e: spin proxy, call from `reqwest` with OpenAI client shape, assert byte-perfect response |

## DRY / SOLID Notes

- **SRP:** the server only routes; translators only translate; backend
  resolution only resolves. Each module has one job.
- **OCP:** adding a new backend means adding one `translate/<backend>.rs`
  + registering its alias in `BackendResolver`. No changes to the server.
- **DIP:** the proxy depends on the `LLMProvider` trait, not on concrete
  providers. Same trait used by the agent's ReAct loop → DRY.
- **ISP:** `OpenAiTranslator` exposes two free functions; we resist the
  urge to make a trait with 12 methods.
- **Security:** localhost bind by default; `--allow-public` requires the
  user to set a non-default token; refuses to start if token unset.

## Streaming Translation Notes

The hard parts:

- **Anthropic → OpenAI:** Anthropic emits `message_start`, `content_block_start`,
  `content_block_delta` (with `text` or `input_json_delta`),
  `content_block_stop`, `message_delta`, `message_stop`. Translator
  buffers `input_json_delta` chunks per `tool_use` block and re-emits as
  OpenAI `tool_calls[*].function.arguments` deltas.
  **CRITICAL** (from prior user memory): flush the final buffered SSE
  `data:` line at EOF even when stream ends without trailing newline.
- **OpenAI → OpenAI (pass-through):** trivial; just forward bytes.
- **Gemini → OpenAI:** non-SSE chunked JSON → re-emit as SSE deltas.

## Error Mapping

| Backend error | OpenAI error code |
|---------------|-------------------|
| 401 (any) | 401 invalid_api_key |
| 429 rate limit | 429 rate_limit_exceeded |
| 5xx | 500 internal_error |
| Anthropic `overloaded_error` | 503 service_unavailable |

Error response body matches OpenAI shape:
```json
{ "error": { "message": "...", "type": "rate_limit_exceeded", "code": "..." } }
```

## Security Posture

- Default bind `127.0.0.1` only.
- Token file `chmod 0600`; missing token → server refuses to start with
  an actionable error.
- `--allow-public` requires explicit flag + a non-default token.
- All inbound requests logged at debug level with token redacted.
- No CORS by default (browser-origin requests rejected); opt-in via
  `proxy.cors_allow_origins`.
- Outbound provider calls respect `HTTPS_PROXY` / `HTTP_PROXY` env vars
  (see `edgecrab-security`).

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Backend prerequisites: [../024-oauth-providers/](../024-oauth-providers/)
