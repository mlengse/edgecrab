# 008 — Security & Trust

What each runtime assumes about the user, the model, and the network.

---

## Defense layers (both implement)

| Layer | Hermes | EdgeCrab |
|-------|--------|----------|
| Path traversal jail | `file_operations.py` | `edgecrab-security/path_jail.rs` |
| SSRF on URLs | `url_safety.py` | `url_safety.rs`, hardened HTTP client |
| Command injection scan | `approval.py` patterns | `command_scan.rs` (~30 patterns) |
| Context file injection | `memory_manager.sanitize_context` | `injection.rs` → block file |
| Memory write injection | Yes | Yes |
| Output redaction | Yes | `redact.rs` pipeline |
| Skills external scan | `skills_guard.py` | `skills_guard.rs` (23+ patterns) |
| Cron prompt injection | `cron/scheduler.py` | `edgecrab-cron/scan.rs` |
| MCP env filtering | `mcp_tool.py` | MCP client isolation |
| Gateway webhook crypto | Twilio, Weixin XML | Same modules |

**Verdict:** **Parity on baseline** — both take security seriously.

---

## Approval modes

| Mode | Hermes | EdgeCrab |
|------|--------|----------|
| manual | Yes | Yes |
| smart (LLM risk score) | Yes | **No** |
| off / yolo | `/yolo`, `HERMES_YOLO_MODE` | `/yolo` |
| Hard blocklist (always on) | `UNRECOVERABLE_BLOCKLIST` | Command scan floor |
| Cron headless mode | `approvals.cron_mode` | Similar |
| Native confirm UI (gateway) | Partial | Gap 015 |

**Verdict:** **Hermes leads UX sophistication** (smart mode); **EdgeCrab relies on static patterns + LSP/shadow verification**.

---

## Write approval gates (Hermes-only pattern)

| Surface | Hermes | EdgeCrab |
|---------|--------|----------|
| Memory writes | Staging + `/memory approve` | Direct write after scan |
| Skill installs | Staging + `/skills approve` | Quarantine→scan→install |
| Dashboard env writer denylist | `_ENV_VAR_NAME_DENYLIST` | N/A (no dashboard) |

**Verdict:** **Hermes leads** for operators who want human review before persistence.

---

## EdgeCrab-only trust mechanisms

| Mechanism | Module | Purpose |
|-----------|--------|---------|
| Shadow judge | `shadow_judge.rs` | LLM verifies task completion |
| File mutation footer | `mutations.rs` | Ground-truth A/M/D per turn |
| LSP write gate | `lsp_gate.rs` | Block/commit on type errors |
| Edit contract limits | `edit_contract.rs` | Cap patch payload sizes |
| Plugin bundle guard | `edgecrab-plugins/guard.rs` | Scan external plugins |
| Steering injection scan | `steering.rs` | Scan steer text |

**Verdict:** **EdgeCrab leads automated coding integrity**; tradeoff is complexity + extra LLM calls.

---

## Supply chain & audit

| | Hermes | EdgeCrab |
|---|--------|----------|
| `doctor` / health | `hermes doctor` | `edgecrab doctor` |
| Security audit CLI | `hermes security audit` + OSV | `doctor` subset |
| Dependency scanning | `tools/osv_check.py` | `cargo audit` (manual) |
| Tirith patterns | `tirith_security.py` | Partial overlap in injection |

**Verdict:** **Hermes leads supply-chain tooling** (integrated OSV audit).

---

## Secrets management

| | Hermes | EdgeCrab |
|---|--------|----------|
| `.env` file | Yes | Yes |
| Bitwarden integration | `hermes secrets` | Gap 032 |
| MCP token store | Yes | `~/.edgecrab/mcp-tokens/` chmod 600 |
| Redaction in logs | Yes | Yes |

**Verdict:** **Hermes leads** secrets UX; **parity on local secret files**.

---

## Threat model honesty

Both agents ** intentionally run arbitrary shell commands** when approved. Neither is a sandbox by default unless docker/modal/ssh backend selected.

**Brutal truth:** Smart approval reduces accidents; it does not eliminate prompt injection → tool exfil chains. EdgeCrab shadow judge adds another LLM judgment layer — not a proof.

---

## Grades

| Dimension | Hermes | EdgeCrab |
|-----------|--------|----------|
| Baseline guards | A | A |
| Human-in-the-loop | A | B |
| Automated coding checks | B | A− |
| Supply chain | A− | B |
| Secrets | A− | B |
| Gateway auth | A | A |

Cross-ref: [001-gap-analysis 015/019/032](../001-gap-analysis-v14/999-roadmap.md)
