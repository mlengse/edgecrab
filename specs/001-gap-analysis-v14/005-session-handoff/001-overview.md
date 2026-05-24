# 005 — Session Handoff (Hermes parity) + Model Transfer

**Tier:** S | **Impact:** 4 | **Value-per-Effort:** 4 | **Risk:** 2  
**Primitive moved:** Reliability + UX + Cost

## Two distinct features (do not conflate)

Hermes uses the word **handoff** for **cross-platform session transfer**. EdgeCrab previously overloaded `/handoff` for **model transfer with brief**. They are now **separate commands**:

| Command | Scope | What it does |
|---------|-------|--------------|
| **`/handoff <platform>`** | CLI only | Transfer the live CLI session to a gateway platform home channel (Telegram, Discord, Slack). Hermes parity. |
| **`/transfer-model <provider/model>`** | CLI + gateway | Alias for `/model` with args — same model-transfer pipeline |

## Why `/handoff` matters (Hermes)

You're deep in a CLI session and want to continue on your phone in Telegram without losing goals, history, or session id. `/handoff telegram`:

1. Validates the gateway is configured with a home channel (`/sethome`).
2. Marks the session `pending` in SQLite.
3. The gateway watcher claims it, re-binds the destination chat to the CLI `session_id`, and runs a synthetic confirmation turn on the platform.
4. The CLI exits cleanly; resume later with `/resume`.

## Why `/transfer-model` matters (EdgeCrab)

Sometimes Opus is overkill for the next 30 turns, or you hit a rate limit and want a cheaper model. **`/model <provider/model>`** and **`/transfer-model`** both run the same pipeline:

- In-flight task brief (auxiliary LLM + structural fallback)
- Context-window safety (auto-compress or refuse)
- Auditable history in `/insights` (`model_transfers` table)
- Cache-safe prompt invalidation

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Depends on: [004-prompt-prefix-cache/](../004-prompt-prefix-cache/) (model transfer)
- Composes with: [001-persistent-goals/](../001-persistent-goals/) (both features preserve goals)
