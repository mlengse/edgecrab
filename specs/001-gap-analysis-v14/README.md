# EdgeCrab ↔ Nous Hermes Agent — Gap Analysis (v0.13 + v0.14, refreshed against v0.15)

> **Mission:** Identify the highest-value features that Nous Hermes Agent ships and that
> EdgeCrab is **missing or only partially implementing**, then rank them so EdgeCrab can
> reach feature + implementation parity.
>
> **Refresh (this pass):** statuses re-verified against the current codebase ("code is
> law"). Tier S is now fully shipped; folders 010–011 shipped; three new gaps from Hermes
> **v0.15** added as Tier D ([031](031-promptware-brainworm-defense/),
> [032](032-secrets-manager/), [033](033-skill-bundles/)).

**Methodology:** See [000-methodology.md](000-methodology.md)
**Sequenced execution roadmap:** See [999-roadmap.md](999-roadmap.md)

**Status legend:** ✅ DONE (verified in code, often with `proof/`) · ◐ PARTIAL (primitive exists, gap remains) · ○ OPEN (not started)

---

## 0. Implementation Status Snapshot (code-is-law, this refresh)

| Status | Count | Folders |
|--------|-------|---------|
| ✅ DONE | 8 | 001, 002, 003, 004, 005, 006, 010, 011 |
| ◐ PARTIAL | 3 | 015 (broker exists, no native inline keyboard), 017 (write/context scan exists, no tool-output scan — see 031), 024 (Copilot+MCP OAuth done, subscription OAuth missing) |
| ○ OPEN | 22 | 007–009, 012–014, 016, 018–023, 025–030, 031–033 |

DONE folders carry a `proof/implementation-proof.md` (except 010, verified directly
in `mcp_oauth.rs` + `JoinSet` dispatch in `conversation.rs`).

### Upstream audit — DONE items vs Hermes v0.15.1 source

Each DONE item was re-verified against the live Hermes checkout
(`/Users/raphaelmansuy/Github/03-working/hermes-agent`, v0.15.1, commit
`827ce602d`) so the `002-hermes-reference.md` claims are themselves
grounded ("code is law" on **both** sides).

| # | EdgeCrab (Rust) | Hermes upstream proof | Verdict |
|---|-----------------|-----------------------|---------|
| 001 | `goals/mod.rs` | `hermes_cli/goals.py` — Ralph loop, `DEFAULT_MAX_TURNS=20`, judge call, state keyed `goal:<session_id>`, no prompt mutation | ✅ matches |
| 002 | `tools/mutations.rs` | `agent/conversation_loop.py` + `agent/tool_executor.py` — `_record_file_mutation_result`, `_turn_failed_file_mutations`, `_format_file_mutation_failure_footer` | ✅ matches (EdgeCrab adds a **success** footer Hermes lacks) |
| 003 | LSP write diagnostics | `agent/lsp/` package | ✅ matches |
| 004 | `prompt_builder.rs` cache | `agent/prompt_caching.py` — `system_and_3`, ephemeral `5m`/`1h` | ✅ matches (EdgeCrab default `1h` **cross-session**; Hermes default `5m` single-session) |
| 005 | `session_handoff.rs` + `platform_handoff.rs` | `cli.py::_handle_handoff_command`, `gateway/run.py::_handoff_watcher`, `claim/complete/fail_handoff`, per-platform `create_handoff_thread` | ✅ matches |
| 006 | `tools/checkpoint.rs` | `tools/checkpoint_manager.py` — single shared shadow-git store, `refs/hermes/<hash16>`, `prune_checkpoints`, `gc --prune=now`, size guard, `retention_days`, legacy migration | ✅ matches (true v2) |
| 010 | `mcp_oauth.rs` + `JoinSet` dispatch | `tools/mcp_oauth.py` (OAuth 2.1 + PKCE), `tool_executor.py::execute_tool_calls_concurrent` (thread pool) | ✅ matches (EdgeCrab uses async `JoinSet` vs Hermes thread pool) |
| 011 | `tools/computer_use/` | `tools/computer_use/` — `cua_backend.py`, `vision_routing.py`, `schema.py`, `tool.py` | ✅ matches |

**Divergences worth noting (EdgeCrab ≥ Hermes):** 002 emits both
success+failure footers (Hermes failure-only); 004 defaults to a 1h
*cross-session* prefix cache (Hermes 5m, single-session); 010 uses async
`JoinSet` rather than a thread pool. No DONE item was found to be weaker
than its Hermes counterpart.

The same audit confirmed the three new Tier-D gaps exist upstream:
`agent/skill_bundles.py` (033), `agent/secret_sources/` (032), and the
consolidated threat scanning that motivates 031.

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

| # | Feature | Primitive moved | Status | Folder |
|---|---------|----------------|--------|--------|
| 001 | Persistent goals (`/goal` Ralph loop + `/subgoal`) | Reliability | ✅ DONE | [001-persistent-goals/](001-persistent-goals/) |
| 002 | Per-turn file-mutation verifier footer | Trust | ✅ DONE | [002-file-mutation-verifier/](002-file-mutation-verifier/) |
| 003 | LSP semantic diagnostics on `write_file`/`patch_file` | Trust | ✅ DONE | [003-lsp-write-diagnostics/](003-lsp-write-diagnostics/) |
| 004 | Cross-session 1h Anthropic prompt cache | Cost | ✅ DONE | [004-prompt-prefix-cache/](004-prompt-prefix-cache/) |
| 005 | `/handoff` live session transfer | Reliability + Cost | ✅ DONE | [005-session-handoff/](005-session-handoff/) |
| 006 | Checkpoints v2 (pruning + disk guardrails) | Trust | ✅ DONE | [006-checkpoints-v2/](006-checkpoints-v2/) |

### TIER A — High-value builds (high impact, higher difficulty)

| # | Feature | Primitive moved | Status | Folder |
|---|---------|----------------|--------|--------|
| 007 | Multi-agent Kanban (durable board + workers) | Reliability | ○ OPEN | [007-multi-agent-kanban/](007-multi-agent-kanban/) |
| 008 | OpenAI-compatible local proxy (`edgecrab proxy`) | Cost | ○ OPEN | [008-openai-compat-proxy/](008-openai-compat-proxy/) |
| 009 | Pluggable `ProviderProfile` + plugin `tool_override` + `ctx.llm` | Reliability | ○ OPEN | [009-pluggable-providers-plugins/](009-pluggable-providers-plugins/) |
| 010 | MCP SSE transport + OAuth forwarding + parallel tool calls | Cost + Trust | ✅ DONE | [010-mcp-sse-oauth-parallel/](010-mcp-sse-oauth-parallel/) |
| 011 | `computer_use` tool (cua-driver, provider-agnostic) | Reliability | ✅ DONE | [011-computer-use/](011-computer-use/) |
| 012 | `video_analyze` + pluggable `video_generate` | Reliability | ○ OPEN | [012-video-tools/](012-video-tools/) |

### TIER B — Quick wins (medium impact, low difficulty)

| # | Feature | Primitive moved | Status | Folder |
|---|---------|----------------|--------|--------|
| 013 | `no_agent` cron mode (script-only watchdog) | Cost | ○ OPEN | [013-cron-no-agent/](013-cron-no-agent/) |
| 014 | Web search backends: SearXNG + Brave + DDGS, per-capability split | Cost | ○ OPEN | [014-web-search-backends/](014-web-search-backends/) |
| 015 | Inline keyboard buttons for `clarify` on Telegram/Discord | Reliability | ◐ PARTIAL | [015-native-clarify-buttons/](015-native-clarify-buttons/) |
| 016 | Discord channel history backfill | Reliability | ○ OPEN | [016-discord-history-backfill/](016-discord-history-backfill/) |
| 017 | Tool error sanitisation (prompt-injection scan on error strings) | Trust | ◐ PARTIAL | [017-tool-error-sanitization/](017-tool-error-sanitization/) |
| 018 | OSC8 clickable URLs in TUI | Cost (UX) | ○ OPEN | [018-osc8-clickable-urls/](018-osc8-clickable-urls/) |
| 019 | Sudo brute-force block + dangerous-command bypass closures | Trust | ○ OPEN | [019-sudo-bruteforce-defense/](019-sudo-bruteforce-defense/) |
| 020 | Gateway session auto-resume after restart | Reliability | ○ OPEN | [020-session-auto-resume/](020-session-auto-resume/) |

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
| 029 | OpenRouter Pareto Code router (`min_coding_score`) | Cost | ○ OPEN | [029-pareto-code-router/](029-pareto-code-router/) |
| 030 | Plugin `transform_llm_output` hook | Reliability | ○ OPEN | [030-transform-llm-output-hook/](030-transform-llm-output-hook/) |

### TIER D — v0.15 Refresh (new gaps surfaced this pass)

| # | Feature | Primitive moved | Status | Folder |
|---|---------|----------------|--------|--------|
| 031 | Promptware / Brainworm defense (3 chokepoints + single threat source) | Trust | ○ OPEN | [031-promptware-brainworm-defense/](031-promptware-brainworm-defense/) |
| 032 | External secrets manager (Bitwarden-class, one bootstrap token) | Trust | ○ OPEN | [032-secrets-manager/](032-secrets-manager/) |
| 033 | Skill bundles (`/<name>` loads many skills at once) | Reliability | ○ OPEN | [033-skill-bundles/](033-skill-bundles/) |

> **Why 031 is Tier S-grade despite being new:** it consolidates four
> drifting threat-pattern lists into one source *and* closes the two
> undefended context chokepoints (tool output, recalled memory). It also
> supersedes the narrower [017-tool-error-sanitization/](017-tool-error-sanitization/).

---

## 4. What EdgeCrab Already Matches (no action needed)

To avoid wasted work, the following Hermes v0.13–v0.15 features have *equivalent* EdgeCrab
implementations and are **out of scope**:

| Hermes feature | EdgeCrab equivalent |
|---------------|---------------------|
| Persistent goals / Ralph loop        | `crates/edgecrab-core/src/goals/mod.rs` — **shipped** (see 001 proof) |
| MCP SSE transport + OAuth + parallel tool calls | `mcp_oauth.rs` + `JoinSet` dispatch in `conversation.rs` — **shipped** (010) |
| `computer_use` (cua-driver)          | `crates/edgecrab-tools/src/tools/computer_use/` — **shipped** (011) |
| Deliverable mode (auto-attach output)| `MEDIA://` protocol in `DeliveryRouter` (gateway) |
| `hermes send` one-shot CLI           | `send_message` tool + gateway `DeliveryRouter` |
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
