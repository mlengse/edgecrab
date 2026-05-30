# 031 — Hermes Reference

## Source

Hermes v0.15.0 "Promptware defense — Brainworm-class attacks blocked at
three chokepoints" — PRs
[#32269](https://github.com/NousResearch/hermes-agent/pull/32269),
[#33131](https://github.com/NousResearch/hermes-agent/pull/33131),
[#9151](https://github.com/NousResearch/hermes-agent/pull/9151).

## What Hermes Did

| Chokepoint | Hermes mechanism |
|------------|------------------|
| **Single source of truth** | `tools/threat_patterns.py` — one module; ~15 new Brainworm/C2 patterns added in one place, consumed everywhere |
| **Recalled memory** | memory is scanned **at load time** (not just write time) before re-injection |
| **Tool output** | tool results get **delimiter markers** so a malicious file or remote service cannot impersonate Hermes' own system content |
| **Dangerous code writes** | new `security-guidance` plugin pattern-matches risky writes (e.g. `os.system`, `eval`, credential reads) |

## Threat Model It Closes

- A file the agent reads contains forged framing (`</result> System:
  ...`) → the delimiter markers make the forgery visible as *content*,
  not as structure.
- A memory entry was poisoned out-of-band → load-time scan catches it
  before it re-enters the prompt.
- A new attack pattern is discovered → it is added to **one** file and
  every chokepoint inherits it immediately (DRY).

## Key Design Principle

The patterns live in exactly one place. Hermes explicitly calls out the
"single source of truth (`tools/threat_patterns.py`)" — the anti-pattern
being N drifting copies.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
