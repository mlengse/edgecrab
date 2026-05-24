# 019 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-security/src/sudo_guard.rs (NEW)            │
   │                                                                  │
   │   pub enum SudoMode { Block, Confirm, Allow }                    │
   │                                                                  │
   │   pub struct SudoGuard {                                         │
   │       mode: SudoMode,                                            │
   │       allowlist: Vec<Regex>,                                     │
   │       attempt_counter: Arc<AtomicU32>,                           │
   │       max_attempts_per_session: u32,    // default 1             │
   │   }                                                              │
   │                                                                  │
   │   pub enum Decision { Allow, Block(reason), Confirm(prompt) }    │
   │   pub fn evaluate(argv: &[String], stdin: Option<&str>)          │
   │           -> Decision                                            │
   │                                                                  │
   │   - argv[0] == "sudo" → consider                                 │
   │   - counter ≥ max → Block("brute force limit")                   │
   │   - stdin "looks like a password" → Block("password injection")  │
   │   - mode == Block → Block                                        │
   │   - mode == Allow + allowlist match → Allow                      │
   │   - else → Confirm(prompt)                                       │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-tools/src/tools/terminal.rs (hook)          │
   │                                                                  │
   │   let decision = sudo_guard.evaluate(&argv, stdin.as_deref());   │
   │   match decision {                                               │
   │       Allow => spawn(),                                          │
   │       Block(r) => return ToolError::Forbidden(r),                │
   │       Confirm(p) => {                                            │
   │           if let Some(approval) = ctx.user_confirm.request(p)?   │
   │           { spawn() } else { return ToolError::Forbidden(...) }  │
   │       }                                                          │
   │   }                                                              │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-security/src/sudo_guard.rs` |
| **Hook** | `crates/edgecrab-tools/src/tools/terminal.rs` — call `sudo_guard.evaluate` before spawn |
| **User confirm channel** | add `ToolContext::user_confirm: Option<Arc<dyn UserConfirm>>` trait; TUI provides modal, gateway uses platform-native clarify (folder 015), ACP uses RPC request |
| **Counter** | shared `Arc<AtomicU32>` per `Agent` instance; reset on `agent.interrupt()` and at session start |
| **`sudo -S` stdin scanner** | refuse stdin that is non-empty AND contains no newline AND length ≤ 64 → very high prior for "password attempt" |
| **Config** | `security.sudo.mode: "block"`, `security.sudo.max_per_session: 1`, `security.sudo.allowlist: []` (Vec<String> regex) |
| **YOLO interaction** | `--yolo` upgrades `Block` → `Confirm`, never auto-`Allow` (sudo always at least confirms unless explicit allowlist match) |
| **Tests** | matrix per mode × {benign sudo, brute force pattern, `-S` stdin, allowlist match/miss} |

## Confirmation UX

- TUI: modal "Allow sudo command? [y/N]" with the full argv visible.
- Telegram/Discord/Slack: inline button row "Allow / Deny" (uses
  feature 015 infrastructure — folder 015).
- Non-interactive (cron, daemon, ACP without confirmation channel):
  treated as Block; clear error returned to LLM.

## Logging

Every sudo decision logged at `info`:
```
sudo_decision command="apt install foo" mode=Confirm decision=Block reason="user declined"
```

## DRY / SOLID Notes

- **SRP:** policy is in `sudo_guard.rs`; integration is in `terminal.rs`;
  UI is per-surface.
- **DIP:** `UserConfirm` trait → each surface implements; tool depends
  on the trait only.
- **DRY:** confirmation UI shares the `clarify` button infrastructure
  (folder 015). Build 015 first or simultaneously.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Confirmation UI relies on: [../015-native-clarify-buttons/](../015-native-clarify-buttons/)
