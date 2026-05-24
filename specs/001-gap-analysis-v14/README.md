# EdgeCrab ↔ Nous Hermes Agent — Gap Analysis (v0.13 + v0.14)

> **Mission:** Identify the highest-value features that Nous Hermes Agent ships in
> `v0.13.0` and `v0.14.0` and that EdgeCrab is **missing or only partially implementing**,
> then rank them so EdgeCrab can reach feature + implementation parity.

**Methodology:** See [000-methodology.md](000-methodology.md)
**Sequenced execution roadmap:** See [999-roadmap.md](999-roadmap.md)

---

## 1. First Principles Framing

Three primitives determine an agent platform's perceived quality:

1. **Reliability of long-horizon execution** — does the agent stay on goal across many turns?
2. **Trust in side-effects** — when the agent edits files / runs commands, can you verify it?
3. **Cost per useful turn** — measured in $, tokens, latency, *and* developer attention.

Every Hermes v0.13/v0.14 feature maps to at least one of those primitives. The ranking
below prioritises features that move ≥ 2 of the 3 primitives at low/medium implementation cost.

---

## 2. Tier Matrix

```
                IMPACT
                Low          Medium         High
              ┌────────────┬────────────┬─────────────┐
        High  │            │            │  TIER C     │
              │            │            │  (strategic │
              │            │            │   bets)     │
DIFFICULTY    ├────────────┼────────────┼─────────────┤
        Med   │            │  TIER B    │  TIER A     │
              │            │  (quick    │  (high-val  │
              │            │   wins)    │   builds)   │
              ├────────────┼────────────┼─────────────┤
        Low   │   skip     │  TIER B    │  TIER S     │
              │            │            │  (do first) │
              └────────────┴────────────┴─────────────┘
```

---

## 3. Ranked Feature Index

Each row links to a feature folder containing 5 cross-referenced documents:
`001-overview.md`, `002-hermes-reference.md`, `003-edgecrab-current-state.md`,
`004-implementation-plan.md`, `005-acceptance-criteria.md`.

### TIER S — Do First (high impact, low/medium difficulty)

| # | Feature | Primitive moved | Folder |
|---|---------|----------------|--------|
| 001 | Persistent goals (`/goal` Ralph loop + `/subgoal`) | Reliability | [001-persistent-goals/](001-persistent-goals/) |
| 002 | Per-turn file-mutation verifier footer | Trust | [002-file-mutation-verifier/](002-file-mutation-verifier/) |
| 003 | LSP semantic diagnostics on `write_file`/`patch_file` | Trust | [003-lsp-write-diagnostics/](003-lsp-write-diagnostics/) |
| 004 | Cross-session 1h Anthropic prompt cache | Cost | [004-prompt-prefix-cache/](004-prompt-prefix-cache/) |
| 005 | `/handoff` live session transfer | Reliability + Cost | [005-session-handoff/](005-session-handoff/) |
| 006 | Checkpoints v2 (pruning + disk guardrails) | Trust | [006-checkpoints-v2/](006-checkpoints-v2/) |

### TIER A — High-value builds (high impact, higher difficulty)

| # | Feature | Primitive moved | Folder |
|---|---------|----------------|--------|
| 007 | Multi-agent Kanban (durable board + workers) | Reliability | [007-multi-agent-kanban/](007-multi-agent-kanban/) |
| 008 | OpenAI-compatible local proxy (`edgecrab proxy`) | Cost | [008-openai-compat-proxy/](008-openai-compat-proxy/) |
| 009 | Pluggable `ProviderProfile` + plugin `tool_override` + `ctx.llm` | Reliability | [009-pluggable-providers-plugins/](009-pluggable-providers-plugins/) |
| 010 | MCP SSE transport + OAuth forwarding + parallel tool calls | Cost + Trust | [010-mcp-sse-oauth-parallel/](010-mcp-sse-oauth-parallel/) |
| 011 | `computer_use` tool (cua-driver, provider-agnostic) | Reliability | [011-computer-use/](011-computer-use/) |
| 012 | `video_analyze` + pluggable `video_generate` | Reliability | [012-video-tools/](012-video-tools/) |

### TIER B — Quick wins (medium impact, low difficulty)

| # | Feature | Primitive moved | Folder |
|---|---------|----------------|--------|
| 013 | `no_agent` cron mode (script-only watchdog) | Cost | [013-cron-no-agent/](013-cron-no-agent/) |
| 014 | Web search backends: SearXNG + Brave + DDGS, per-capability split | Cost | [014-web-search-backends/](014-web-search-backends/) |
| 015 | Inline keyboard buttons for `clarify` on Telegram/Discord | Reliability | [015-native-clarify-buttons/](015-native-clarify-buttons/) |
| 016 | Discord channel history backfill | Reliability | [016-discord-history-backfill/](016-discord-history-backfill/) |
| 017 | Tool error sanitisation (prompt-injection scan on error strings) | Trust | [017-tool-error-sanitization/](017-tool-error-sanitization/) |
| 018 | OSC8 clickable URLs in TUI | Cost (UX) | [018-osc8-clickable-urls/](018-osc8-clickable-urls/) |
| 019 | Sudo brute-force block + dangerous-command bypass closures | Trust | [019-sudo-bruteforce-defense/](019-sudo-bruteforce-defense/) |
| 020 | Gateway session auto-resume after restart | Reliability | [020-session-auto-resume/](020-session-auto-resume/) |

### TIER C — Strategic bets (high impact, high difficulty)

| # | Feature | Primitive moved | Folder |
|---|---------|----------------|--------|
| 021 | Curator (skill consolidation + archive/prune subcommands) | Cost | [021-curator-subsystem/](021-curator-subsystem/) |
| 022 | Cold-start perf wave (lazy imports + disk-cache-first catalogs + parallel doctor) | Cost (UX) | [022-cold-start-perf/](022-cold-start-perf/) |
| 023 | 180× faster `browser_console` via persistent CDP WebSocket | Cost | [023-persistent-cdp-ws/](023-persistent-cdp-ws/) |
| 024 | xAI Grok OAuth (SuperGrok) + Claude Pro / ChatGPT Pro OAuth providers | Cost | [024-oauth-providers/](024-oauth-providers/) |
| 025 | i18n (16 locales for gateway + CLI static messages) | Reach | [025-i18n/](025-i18n/) |
| 026 | New gateway platforms: LINE, SimpleX, Google Chat, MS Teams | Reach | [026-new-platforms/](026-new-platforms/) |
| 027 | `x_search` Twitter tool | Reliability | [027-x-search-tool/](027-x-search-tool/) |
| 028 | Skills Hub: huggingface trusted tap + per-skill pages | Reach | [028-skills-hub-trusted-taps/](028-skills-hub-trusted-taps/) |
| 029 | OpenRouter Pareto Code router (`min_coding_score`) | Cost | [029-pareto-code-router/](029-pareto-code-router/) |
| 030 | Plugin `transform_llm_output` hook | Reliability | [030-transform-llm-output-hook/](030-transform-llm-output-hook/) |

---

## 4. What EdgeCrab Already Matches (no action needed)

To avoid wasted work, the following Hermes v0.13/v0.14 features have *equivalent* EdgeCrab
implementations and are **out of scope**:

| Hermes feature | EdgeCrab equivalent |
|---------------|---------------------|
| `/steer` (mid-turn steering)         | `Agent::steer_sender()` + `SteeringEvent` (`crates/edgecrab-core/src/agent.rs`) |
| `/queue` (next-turn enqueue)         | `gateway.second_message_mode: queue` |
| Sessions DB (FTS search)             | `edgecrab-state` SQLite WAL + FTS5 |
| Structural + LLM compression         | `crates/edgecrab-core/src/compression.rs` |
| Skills sync from manifest            | `crates/edgecrab-tools/src/tools/skills_sync.rs` |
| ACP adapter (VS Code)                | `crates/edgecrab-acp/` |
| Honcho (long-term memory)            | `crates/edgecrab-tools/src/tools/honcho.rs` (analogous to Hindsight) |
| Skills hub + skills guard            | `skills_hub.rs` + `skills_guard.rs` |
| Mission Steering (HINT/REDIRECT/STOP)| `SteeringKind` enum |
| Termux/Android compact UI            | `IS_TERMUX` + `BasicCompat` UI profile |

---

## 5. How to Use This Document

1. Read [000-methodology.md](000-methodology.md) for scoring criteria.
2. Pick a tier; read each folder's `001-overview.md` to confirm the gap is real.
3. Use `004-implementation-plan.md` as the Rust execution blueprint.
4. Verify completion via `005-acceptance-criteria.md`.
5. Sequence via [999-roadmap.md](999-roadmap.md).
