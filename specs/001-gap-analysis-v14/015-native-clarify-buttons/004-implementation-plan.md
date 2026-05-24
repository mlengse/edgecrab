# 015 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-tools/src/tools/clarify.rs                        │
   │                                                                  │
   │   ClarifyArgs {                                                  │
   │     question: String,                                            │
   │     options: Vec<ClarifyOption>   ← NEW structured field         │
   │   }                                                              │
   │   ClarifyOption { label, value, emoji?, style? }                 │
   │                                                                  │
   │   Output format:                                                 │
   │   "CLARIFY://{base64(json{question, options})}"                  │
   │   plus a human-readable fallback line for text-only platforms    │
   └──────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-gateway/src/clarify_router.rs (NEW)               │
   │                                                                  │
   │   intercept CLARIFY:// in DeliveryRouter                         │
   │   decode → ClarifyPayload                                        │
   │   ask platform adapter via trait method:                         │
   │     async fn send_clarify(&self, target, payload) -> Sent;       │
   │                                                                  │
   │   adapter default impl: render as numbered text                  │
   │   Telegram/Discord/Slack override with native widgets            │
   └──────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
   ┌──────────────────────────────────────────────────────────────────┐
   │  Per-platform callback ingest                                    │
   │                                                                  │
   │  Telegram: handle update.callback_query → synthesise user msg   │
   │  Discord: interaction endpoint (HTTP) → synthesise user msg     │
   │  Slack: block_actions payload → synthesise user msg             │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Modify** | `crates/edgecrab-tools/src/tools/clarify.rs` — add `options` arg + `CLARIFY://` output sentinel |
| **New** | `crates/edgecrab-gateway/src/clarify_router.rs` — sentinel interception |
| **Trait extension** | `PlatformAdapter` gains `async fn send_clarify(&self, target, payload) -> Result<MessageId>` with default that calls `send_text` with numbered fallback |
| **Telegram impl** | `crates/edgecrab-gateway/src/platforms/telegram.rs` — `inline_keyboard` per row of 2 |
| **Telegram callback** | same file — handle `callback_query`; `bot.answer_callback_query`; synthesise user message with `option.value` |
| **Discord impl** | `crates/edgecrab-gateway/src/platforms/discord.rs` — `components: [action_row]` |
| **Discord interaction** | same file — HTTP interaction endpoint or gateway opcode 2 |
| **Slack impl** | `crates/edgecrab-gateway/src/platforms/slack.rs` — block kit `actions` |
| **Tests** | mock adapter; assert text fallback works; per-platform unit tests |

## Encoding Rationale

`CLARIFY://{base64(json)}` mirrors the existing `MEDIA://` sentinel
pattern — `DeliveryRouter` already knows how to intercept sentinel
prefixes. DRY win.

## Fallback Behaviour

Any platform without `send_clarify` override renders:

```
Which option?
  1) Yes  (value: yes)
  2) No   (value: no)
  3) Maybe (value: maybe)
```

User replies "1" or "yes" — both accepted (normalise: trim, lower,
match by index or value).

## DRY / SOLID Notes

- **OCP:** platforms opt-in to rich clarify by overriding one method;
  the rest get text fallback for free.
- **DRY:** sentinel protocol reuses the `MEDIA://` interception pattern.
- **SRP:** clarify_router does interception; per-platform files do
  rendering; tool produces structured data only.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
