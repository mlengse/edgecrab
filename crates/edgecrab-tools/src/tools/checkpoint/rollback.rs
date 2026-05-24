//! Shared `/rollback` command handler (TUI + gateway).

use std::path::Path;

use super::display::format_checkpoint_list;
use super::git::validate_commit_hash;
use super::manager::CheckpointManager;
use super::types::CheckpointConfig;

/// Result of handling a `/rollback` command.
#[derive(Debug, Clone)]
pub enum RollbackOutcome {
    Disabled,
    System(String),
    Error(String),
    Report {
        title: String,
        subtitle: String,
        body: String,
    },
}

/// Parse and execute `/rollback` args against `cwd`.
pub fn handle_rollback_command(
    args: &str,
    cwd: &Path,
    cfg: CheckpointConfig,
) -> RollbackOutcome {
    if !cfg.enabled {
        return RollbackOutcome::Disabled;
    }

    let mgr = CheckpointManager::new(cfg);
    let trimmed = args.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let sub = parts
        .first()
        .map(|p| p.to_ascii_lowercase())
        .unwrap_or_default();

    match sub.as_str() {
        "" | "list" | "ls" => {
            let entries = mgr.list_checkpoints(cwd);
            RollbackOutcome::Report {
                title: "Rollback checkpoints".into(),
                subtitle: "Filesystem snapshots".into(),
                body: format_checkpoint_list(&entries, cwd),
            }
        }
        "pin" | "unpin" => {
            let Some(n_str) = parts.get(1) else {
                return RollbackOutcome::System(format!("Usage: /rollback {sub} <N>"));
            };
            let Ok(n) = n_str.parse::<usize>() else {
                return RollbackOutcome::Error(format!("Invalid checkpoint number: {n_str}"));
            };
            match mgr.pin_checkpoint(cwd, n, sub == "pin") {
                Ok(msg) => RollbackOutcome::System(msg),
                Err(e) => RollbackOutcome::Error(e),
            }
        }
        "diff" => {
            let Some(n_str) = parts.get(1) else {
                return RollbackOutcome::System("Usage: /rollback diff <N>".into());
            };
            let Ok(n) = n_str.parse::<usize>() else {
                return RollbackOutcome::Error(format!("Invalid checkpoint number: {n_str}"));
            };
            match mgr.diff_against_n(cwd, n) {
                Ok(text) if text == "no changes vs current state" => RollbackOutcome::System(text),
                Ok(text) => RollbackOutcome::Report {
                    title: format!("Rollback diff #{n}"),
                    subtitle: "Changes since checkpoint".into(),
                    body: text,
                },
                Err(e) => RollbackOutcome::Error(e),
            }
        }
        _ => {
            let first = parts[0];
            let file_path = parts.get(1).copied();
            if let Ok(n) = first.parse::<usize>() {
                match mgr.restore_by_n(cwd, n, file_path, None) {
                    Ok(result) => {
                        let target = file_path.unwrap_or("workspace");
                        RollbackOutcome::System(format!(
                            "Restored {target} to {} ({}): {} file(s)",
                            result.restored_to, result.reason, result.files_restored
                        ))
                    }
                    Err(e) => RollbackOutcome::Error(format!("Rollback failed: {e}")),
                }
            } else if validate_commit_hash(first).is_none() {
                match mgr.restore(cwd, first, file_path, None) {
                    Ok(result) => {
                        let target = file_path.unwrap_or("workspace");
                        RollbackOutcome::System(format!(
                            "Restored {target} to {} ({}): {} file(s)",
                            result.restored_to, result.reason, result.files_restored
                        ))
                    }
                    Err(e) => RollbackOutcome::Error(format!("Rollback failed: {e}")),
                }
            } else {
                RollbackOutcome::Error(format!(
                    "Unknown rollback command or invalid number/hash: {first}"
                ))
            }
        }
    }
}
