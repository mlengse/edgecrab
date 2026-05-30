//! # checkpoint — Shadow-git filesystem snapshots for rollback (v2)
//!
//! Hermes v2 single shared git store with real pruning, exclude rules, and
//! disk caps. Automatic snapshots fire once per turn before file mutations.

mod display;
mod excludes;
mod git;
mod maintenance;
mod manager;
mod prune;
mod ref_ops;
mod rollback;
mod save;
mod types;

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use edgecrab_types::{ToolError, ToolSchema};

use crate::registry::{ToolContext, ToolHandler};

pub use display::{format_bytes, format_checkpoint_list};
pub use maintenance::{
    ClearLegacyResult, ClearResult, LegacyArchive, StoreProject, StoreStatus, clear_all,
    clear_legacy, format_store_status, store_status,
};
pub use manager::CheckpointManager;
pub use prune::{AutoPruneResult, PruneCounts, maybe_auto_prune_checkpoints, prune_checkpoints};
pub use rollback::{RollbackOutcome, handle_rollback_command};
pub use types::{CheckpointConfig, CheckpointEntry, RestoreResult};

static TURN_DIRS: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();

fn turn_dirs() -> &'static Mutex<std::collections::HashSet<String>> {
    TURN_DIRS.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

/// Reset per-turn deduplication. Call at the start of each agent iteration.
pub fn checkpoint_new_turn() {
    if let Ok(mut guard) = turn_dirs().lock() {
        guard.clear();
    }
}

/// Auto-create a checkpoint before a mutation if checkpoints are enabled.
pub fn ensure_checkpoint(ctx: &ToolContext, reason: &str) {
    if !ctx.config.checkpoints_enabled {
        return;
    }
    let abs = git::normalize_path(&ctx.cwd).to_string_lossy().into_owned();
    {
        let Ok(mut guard) = turn_dirs().lock() else {
            return;
        };
        if guard.contains(&abs) {
            return;
        }
        guard.insert(abs);
    }

    let mut mgr = CheckpointManager::new(CheckpointConfig::from_ctx(ctx));
    let _ = mgr.ensure_checkpoint(&ctx.cwd, reason);
}

fn resolve_checkpoint_n(entries: &[CheckpointEntry], n: u32) -> Option<String> {
    entries
        .iter()
        .find(|e| e.n == n as usize)
        .map(|e| e.hash.clone())
}

fn action_create(ctx: &ToolContext, reason: &str) -> Result<String, ToolError> {
    let mut mgr = CheckpointManager::new(CheckpointConfig::from_ctx(ctx));
    let taken = mgr.ensure_checkpoint(&ctx.cwd, reason);
    let entries = mgr.list_checkpoints(&ctx.cwd);
    let n = entries.first().map(|e| e.n);
    Ok(serde_json::to_string(&json!({
        "ok": true,
        "action": "created",
        "n": n,
        "label": reason,
        "has_new_commit": taken
    }))
    .expect("infallible"))
}

fn action_list(ctx: &ToolContext) -> Result<String, ToolError> {
    let mgr = CheckpointManager::new(CheckpointConfig::from_ctx(ctx));
    let entries = mgr.list_checkpoints(&ctx.cwd);
    let checkpoints: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            json!({
                "n": e.n,
                "hash": crate::safe_truncate(&e.short_hash, 8),
                "label": e.reason,
                "date": e.timestamp,
                "size_bytes": e.size_bytes,
                "pinned": e.pinned,
                "files_changed": e.files_changed,
            })
        })
        .collect();
    Ok(serde_json::to_string(&json!({
        "ok": true,
        "action": "list",
        "total": checkpoints.len(),
        "checkpoints": checkpoints
    }))
    .expect("infallible"))
}

fn action_restore(ctx: &ToolContext, n: u32) -> Result<String, ToolError> {
    let mgr = CheckpointManager::new(CheckpointConfig::from_ctx(ctx));
    let entries = mgr.list_checkpoints(&ctx.cwd);
    let hash = resolve_checkpoint_n(&entries, n).ok_or_else(|| ToolError::ExecutionFailed {
        tool: "checkpoint".into(),
        message: format!("Checkpoint #{n} not found"),
    })?;
    let result =
        mgr.restore(&ctx.cwd, &hash, None, Some(ctx))
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "checkpoint".into(),
                message: e,
            })?;
    Ok(serde_json::to_string(&json!({
        "ok": true,
        "action": "restored",
        "n": n,
        "files_restored": result.files_restored
    }))
    .expect("infallible"))
}

fn action_pin(ctx: &ToolContext, n: u32, pin: bool) -> Result<String, ToolError> {
    let mgr = CheckpointManager::new(CheckpointConfig::from_ctx(ctx));
    let msg =
        mgr.pin_checkpoint(&ctx.cwd, n as usize, pin)
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "checkpoint".into(),
                message: e,
            })?;
    Ok(serde_json::to_string(&json!({
        "ok": true,
        "action": if pin { "pinned" } else { "unpinned" },
        "n": n,
        "message": msg
    }))
    .expect("infallible"))
}

fn action_diff(ctx: &ToolContext, n: u32) -> Result<String, ToolError> {
    let mgr = CheckpointManager::new(CheckpointConfig::from_ctx(ctx));
    let entries = mgr.list_checkpoints(&ctx.cwd);
    let hash = resolve_checkpoint_n(&entries, n).ok_or_else(|| ToolError::ExecutionFailed {
        tool: "checkpoint".into(),
        message: format!("Checkpoint #{n} not found"),
    })?;

    let base = git::checkpoint_base(&ctx.config.edgecrab_home);
    let store = git::store_path(&base);
    let abs = git::normalize_path(&ctx.cwd);
    let index_file = git::index_path(&store, &git::project_hash(&abs));

    let _ = git::run_git(
        &["add", "-A"],
        &store,
        &abs,
        Some(&index_file),
        &std::collections::HashSet::new(),
        git::GIT_TIMEOUT_SECS * 2,
    );

    let stat = git::run_git(
        &["diff", "--stat", &hash, "--cached"],
        &store,
        &abs,
        Some(&index_file),
        &std::collections::HashSet::new(),
        git::GIT_TIMEOUT_SECS,
    );
    let diff = git::run_git(
        &["diff", &hash, "--cached", "--no-color"],
        &store,
        &abs,
        Some(&index_file),
        &std::collections::HashSet::new(),
        git::GIT_TIMEOUT_SECS,
    );

    let reference = git::ref_name(&git::project_hash(&abs));
    let _ = git::run_git(
        &["read-tree", &reference],
        &store,
        &abs,
        Some(&index_file),
        &std::collections::HashSet::from([128i32]),
        git::GIT_TIMEOUT_SECS,
    );

    if stat.stdout.is_empty() && diff.stdout.is_empty() {
        return Ok(serde_json::to_string(&json!({
            "ok": true,
            "action": "diff",
            "n": n,
            "message": "no changes vs current state"
        }))
        .expect("infallible"));
    }

    Ok(serde_json::to_string(&json!({
        "ok": true,
        "action": "diff",
        "n": n,
        "stat": stat.stdout,
        "diff": diff.stdout
    }))
    .expect("infallible"))
}

fn action_restore_file(ctx: &ToolContext, n: u32, file: &str) -> Result<String, ToolError> {
    let mgr = CheckpointManager::new(CheckpointConfig::from_ctx(ctx));
    let entries = mgr.list_checkpoints(&ctx.cwd);
    let hash = resolve_checkpoint_n(&entries, n).ok_or_else(|| ToolError::ExecutionFailed {
        tool: "checkpoint".into(),
        message: format!("Checkpoint #{n} not found"),
    })?;
    mgr.restore(&ctx.cwd, &hash, Some(file), Some(ctx))
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "checkpoint".into(),
            message: e,
        })?;
    Ok(serde_json::to_string(&json!({
        "ok": true,
        "action": "restore_file",
        "n": n,
        "file": file
    }))
    .expect("infallible"))
}

pub struct CheckpointTool;

#[derive(Deserialize)]
struct CheckpointArgs {
    action: String,
    reason: Option<String>,
    name: Option<String>,
    n: Option<u32>,
    file: Option<String>,
    pin: Option<bool>,
}

#[async_trait]
impl ToolHandler for CheckpointTool {
    fn name(&self) -> &'static str {
        "checkpoint"
    }

    fn toolset(&self) -> &'static str {
        "core"
    }

    fn emoji(&self) -> &'static str {
        "💾"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "checkpoint".into(),
            description: concat!(
                "Create, list, restore, pin, or diff shadow-git filesystem checkpoints. ",
                "Use 'create' before risky changes, 'list' for numbered history (1=newest), ",
                "'restore' to roll back, 'pin' to protect from eviction, ",
                "'diff' to preview changes, 'restore_file' for a single file."
            )
            .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create", "list", "restore", "diff", "restore_file", "pin", "unpin"],
                        "description": "Checkpoint action"
                    },
                    "reason": { "type": "string", "description": "Label for 'create'" },
                    "n": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Checkpoint number (1=newest)"
                    },
                    "file": { "type": "string", "description": "Relative path for restore_file" },
                    "pin": { "type": "boolean", "description": "Pin state for pin/unpin actions" }
                },
                "required": ["action"]
            }),
            strict: None,
        }
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<String, ToolError> {
        let args: CheckpointArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs {
                tool: "checkpoint".into(),
                message: format!("Invalid checkpoint args: {e}"),
            })?;

        if ctx.cancel.is_cancelled() {
            return Err(ToolError::Other("Cancelled".into()));
        }

        let reason = args.reason.or(args.name);

        match args.action.as_str() {
            "create" => {
                let r = reason.unwrap_or_else(|| "manual checkpoint".to_string());
                action_create(ctx, &r)
            }
            "list" => action_list(ctx),
            "restore" => {
                let n = args.n.ok_or_else(|| ToolError::InvalidArgs {
                    tool: "checkpoint".into(),
                    message: "'n' is required for restore".into(),
                })?;
                action_restore(ctx, n)
            }
            "diff" => {
                let n = args.n.ok_or_else(|| ToolError::InvalidArgs {
                    tool: "checkpoint".into(),
                    message: "'n' is required for diff".into(),
                })?;
                action_diff(ctx, n)
            }
            "restore_file" => {
                let n = args.n.ok_or_else(|| ToolError::InvalidArgs {
                    tool: "checkpoint".into(),
                    message: "'n' is required for restore_file".into(),
                })?;
                let file = args.file.ok_or_else(|| ToolError::InvalidArgs {
                    tool: "checkpoint".into(),
                    message: "'file' is required for restore_file".into(),
                })?;
                action_restore_file(ctx, n, &file)
            }
            "pin" | "unpin" => {
                let n = args.n.ok_or_else(|| ToolError::InvalidArgs {
                    tool: "checkpoint".into(),
                    message: "'n' is required for pin/unpin".into(),
                })?;
                let pin = args.action == "pin" || args.pin.unwrap_or(true);
                action_pin(ctx, n, pin)
            }
            other => Err(ToolError::InvalidArgs {
                tool: "checkpoint".into(),
                message: format!(
                    "Unknown action '{other}'. Use: create, list, restore, diff, restore_file, pin, unpin"
                ),
            }),
        }
    }
}

inventory::submit!(&CheckpointTool as &dyn ToolHandler);

#[cfg(test)]
mod tests;
