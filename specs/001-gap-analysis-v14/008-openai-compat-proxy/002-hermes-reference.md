# 008 — Hermes Reference (ground truth)

Hermes agent repo: `hermes-agent/hermes_cli/proxy/`.  
**Only two forward upstreams:** `nous` (Nous Portal) and `xai` (SuperGrok).  
There is **no** Hermes proxy adapter for Claude Pro, ChatGPT Pro, or Copilot.

| Concern | Hermes file |
|---------|-------------|
| CLI | `hermes_cli/proxy/cli.py` — `start`, `status`, `providers` |
| HTTP server | `hermes_cli/proxy/server.py` — aiohttp, default port **8645** |
| Adapters registry | `hermes_cli/proxy/adapters/__init__.py` — `ADAPTERS = {nous, xai}` |
| Nous Portal | `hermes_cli/proxy/adapters/nous_portal.py` — JWT refresh, 401 retry, quarantine, path allowlist |
| xAI Grok | `hermes_cli/proxy/adapters/xai.py` — OIDC refresh, credential pool on 429 |
| Base trait | `hermes_cli/proxy/adapters/base.py` — `UpstreamAdapter`, `UpstreamCredential` |
| OAuth storage | `hermes_cli/auth.py` — `~/.hermes/auth.json`, flock, Nous refresh helpers |

## Wire-Level Behaviour (Mode A only)

```
Client (Aider, Cline, OpenAI SDK)
       │  POST /v1/chat/completions
       │  Authorization: Bearer <any string — ignored for upstream>
       ▼
hermes proxy start [--provider nous|xai]   # default provider: nous
       │  attach real OAuth bearer to upstream
       ▼
Nous inference-api.nousresearch.com  OR  xAI api.x.ai
       ▼
OpenAI-shaped JSON/SSE returned verbatim (no tool-schema translation in proxy)
```

## Local Client Auth

Hermes: **any** Bearer on the client is accepted; the proxy replaces upstream auth.  
EdgeCrab: requires `~/.edgecrab/proxy-token` (or `proxy.token_path`) — stricter same-machine policy.

## Nous Parity Checklist (EdgeCrab `backend/nous/`)

| Hermes behaviour | EdgeCrab |
|------------------|----------|
| Inference URL allowlist | `inference_url.rs` + `NOUS_INFERENCE_BASE_URL` |
| Terminal refresh → quarantine state + pool | `quarantine.rs` |
| `auth.json` cross-process lock | `auth_lock.rs` |
| 401 → force JWT refresh | `adapter.rs` retry credential |
| Allowed paths set | forwarder path checks |

## xAI Parity Checklist

| Hermes behaviour | EdgeCrab |
|------------------|----------|
| OIDC refresh | `backend/xai/refresh.rs` |
| 429 → rotate credential pool | `backend/xai/adapter.rs` |
| `/responses` route | forwarder + adapter base URL |

## EdgeCrab vs Hermes (intentional deltas)

| Topic | Hermes | EdgeCrab |
|-------|--------|----------|
| Default port | 8645 | 11434 (Ollama convention) |
| Default `--provider` | `nous` | none (dual-mode); use `--provider` or `proxy.default_forward_upstream` |
| CLI surface | `start` / `status` / `providers` | + `setup`, `enable`, `doctor`, `client`, `token`, TUI `/proxy` |
| Mode B API-key bridge | not present | `wire/` + `LLMProvider` |
| Provider OAuth login | `hermes auth add …` | Nous: `edgecrab auth add nous`; Grok: `edgecrab auth add grok` (024) |

## Cross-References

- [001-overview.md](001-overview.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
