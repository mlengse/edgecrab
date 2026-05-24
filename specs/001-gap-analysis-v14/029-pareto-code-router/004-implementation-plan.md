# 029 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/router/ (NEW module)                     │
   │                                                                  │
   │   mod.rs                                                         │
   │   classifier.rs    — signals → Tier enum                         │
   │   policy.rs        — Tier → ModelId via config map               │
   │   override.rs      — parse `@<model>` prefix                     │
   │   stats.rs         — per-tier turn counter + cost savings        │
   │                                                                  │
   │   pub enum Tier { Quick, Default, Code, Vision }                 │
   │                                                                  │
   │   pub fn route(msg: &Message, hist: &[Message], cfg: &RouterCfg) │
   │       -> ModelId {                                               │
   │       if let Some(m) = override::parse(msg) { return m; }        │
   │       if !cfg.auto { return cfg.default_model; }                 │
   │       let tier = classifier::classify(msg, hist);                │
   │       policy::map(tier, cfg)                                     │
   │   }                                                              │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-core/src/agent.rs                                  │
   │                                                                  │
   │   before each provider.chat() call:                              │
   │     let model = router::route(latest_msg, &messages, &cfg);      │
   │     stats.record(tier, /*before*/ price_of_default,              │
   │                          /*after*/ price_of(model));             │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-core/src/router/` |
| **Config** | `router.auto: true|false` (default false initially); `router.tiers.quick: "openai/gpt-5-mini"`, `router.tiers.default: "anthropic/claude-sonnet-4.6"`, `router.tiers.code: "anthropic/claude-opus-4.6"`, `router.tiers.vision: "google/gemini-2.5-flash"` |
| **Per-turn override** | parse `^@(\w[\w/-]+)\s+` prefix from user message |
| **Stats** | append per-turn record to `~/.edgecrab/router-stats.jsonl`; `/router stats` summarises last N days |
| **Slash command** | `/router auto on|off`, `/router stats`, `/router show` (current tier mapping) |
| **Streaming** | if mid-stream, do NOT switch models (would break partial context); router decision happens at top of new turn only |
| **Cost diff calc** | use `pricing.rs`; estimate input tokens via `tiktoken` (or heuristic 4 chars/token); compute `(price_default - price_chosen) * estimated_tokens` |
| **Tests** | classifier matrix tests; override parsing; stats accumulation |

## Risks

- Bad classification = bad output. Conservative defaults: `auto: false`
  ships off; user opts in.
- Mid-conversation model switching might lose Anthropic prompt cache.
  Mitigation: persist `current_model_for_session`; only re-route on
  user-message turn boundaries; keep session sticky once `code` tier
  triggered (avoid pong-ing).

## DRY / SOLID Notes

- **SRP:** classifier, policy, override, stats are separate.
- **OCP:** add a tier = enum variant + config key; no logic changes
  elsewhere.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
