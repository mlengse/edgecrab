//! Checkpoint manager — Hermes v2 single shared shadow git store (Rust port).

use std::collections::HashSet;
use std::path::Path;

use tracing::debug;

use crate::mutations::{MutationKind, MutationRecord};
use crate::registry::ToolContext;

use super::git::{
    checkpoint_base, index_path, load_pinned_shas, normalize_path, project_hash, ref_name, run_git,
    set_pin, store_path, validate_commit_hash, validate_file_path, GIT_TIMEOUT_SECS,
};
use super::ref_ops::{estimate_commit_bytes, list_worktree_files, parse_shortstat};
use super::types::{CheckpointConfig, CheckpointEntry, RestoreResult};

/// Manages automatic filesystem checkpoints with per-turn deduplication.
pub struct CheckpointManager {
    pub(crate) cfg: CheckpointConfig,
    checkpointed_dirs: HashSet<String>,
}

impl CheckpointManager {
    pub fn new(cfg: CheckpointConfig) -> Self {
        Self {
            cfg,
            checkpointed_dirs: HashSet::new(),
        }
    }

    pub fn new_turn(&mut self) {
        self.checkpointed_dirs.clear();
    }

    pub fn ensure_checkpoint(&mut self, working_dir: &Path, reason: &str) -> bool {
        if !self.cfg.enabled {
            return false;
        }
        if which::which("git").is_err() {
            debug!("checkpoints disabled: git not found");
            return false;
        }

        let abs = normalize_path(working_dir).to_string_lossy().into_owned();
        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_default();
        if abs == "/" || abs == home {
            debug!("checkpoint skipped: directory too broad ({abs})");
            return false;
        }
        if self.checkpointed_dirs.contains(&abs) {
            return false;
        }
        self.checkpointed_dirs.insert(abs);

        match self.take(working_dir, reason) {
            Ok(v) => v,
            Err(e) => {
                debug!("checkpoint failed (non-fatal): {e}");
                false
            }
        }
    }

    pub fn list_checkpoints(&self, working_dir: &Path) -> Vec<CheckpointEntry> {
        let base = checkpoint_base(&self.cfg.edgecrab_home);
        let store = store_path(&base);
        if !(store.join("HEAD")).exists() {
            return Vec::new();
        }

        let abs = normalize_path(working_dir);
        let reference = ref_name(&project_hash(&abs));
        let allowed_missing = HashSet::from([128i32, 129]);
        let limit = self.cfg.max_snapshots.max(1);

        let result = run_git(
            &[
                "log",
                &reference,
                "--format=%H|%h|%aI|%s",
                "-n",
                &limit.to_string(),
            ],
            &store,
            &abs,
            None,
            &allowed_missing,
            GIT_TIMEOUT_SECS,
        );
        if !result.ok || result.stdout.is_empty() {
            return Vec::new();
        }

        let pinned = load_pinned_shas(&store, &abs);
        let mut out = Vec::new();
        for (i, line) in result.stdout.lines().enumerate() {
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() != 4 {
                continue;
            }
            let mut entry = CheckpointEntry {
                n: i + 1,
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                timestamp: parts[2].to_string(),
                reason: parts[3].to_string(),
                files_changed: 0,
                insertions: 0,
                deletions: 0,
                size_bytes: estimate_commit_bytes(&store, &abs, parts[0]),
                pinned: pinned.contains(parts[0]),
            };

            let stat = run_git(
                &["diff", "--shortstat", &format!("{}~1", parts[0]), parts[0]],
                &store,
                &abs,
                None,
                &allowed_missing,
                GIT_TIMEOUT_SECS,
            );
            if stat.ok && !stat.stdout.is_empty() {
                parse_shortstat(&stat.stdout, &mut entry);
            }
            out.push(entry);
        }
        out
    }

    pub fn pin_checkpoint(&self, working_dir: &Path, n: usize, pin: bool) -> Result<String, String> {
        let entries = self.list_checkpoints(working_dir);
        let entry = entries
            .into_iter()
            .find(|e| e.n == n)
            .ok_or_else(|| format!("Checkpoint #{n} not found"))?;
        let store = store_path(&checkpoint_base(&self.cfg.edgecrab_home));
        set_pin(&store, working_dir, &entry.hash, pin)?;
        Ok(if pin {
            format!("Pinned checkpoint #{n} ({})", entry.short_hash)
        } else {
            format!("Unpinned checkpoint #{n} ({})", entry.short_hash)
        })
    }

    pub fn restore_by_n(
        &self,
        working_dir: &Path,
        n: usize,
        file_path: Option<&str>,
        ctx: Option<&ToolContext>,
    ) -> Result<RestoreResult, String> {
        let hash = self
            .list_checkpoints(working_dir)
            .into_iter()
            .find(|e| e.n == n)
            .map(|e| e.hash)
            .ok_or_else(|| format!("Checkpoint #{n} not found"))?;
        self.restore(working_dir, &hash, file_path, ctx)
    }

    pub fn restore(
        &self,
        working_dir: &Path,
        commit_hash: &str,
        file_path: Option<&str>,
        ctx: Option<&ToolContext>,
    ) -> Result<RestoreResult, String> {
        if let Some(err) = validate_commit_hash(commit_hash) {
            return Err(err);
        }
        if let Some(fp) = file_path
            && let Some(err) = validate_file_path(fp, working_dir)
        {
            return Err(err);
        }

        let base = checkpoint_base(&self.cfg.edgecrab_home);
        let store = store_path(&base);
        if !(store.join("HEAD")).exists() {
            return Err("No checkpoints exist for this directory".into());
        }

        let abs = normalize_path(working_dir);
        let verify = run_git(
            &["cat-file", "-t", commit_hash],
            &store,
            &abs,
            None,
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        if !verify.ok {
            return Err(format!("Checkpoint '{commit_hash}' not found"));
        }

        let short = &commit_hash[..8.min(commit_hash.len())];
        let mut pre = CheckpointManager::new(self.cfg.clone());
        let _ = pre.take(&abs, &format!("pre-rollback snapshot (restoring to {short})"));

        let index_file = index_path(&store, &project_hash(&abs));
        let target = file_path.unwrap_or(".");
        let checkout = run_git(
            &["checkout", commit_hash, "--", target],
            &store,
            &abs,
            Some(&index_file),
            &HashSet::new(),
            GIT_TIMEOUT_SECS * 2,
        );
        if !checkout.ok {
            return Err(format!("Restore failed: {}", checkout.stderr));
        }

        let reason = run_git(
            &["log", "--format=%s", "-1", commit_hash],
            &store,
            &abs,
            None,
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        let reason_text = if reason.ok && !reason.stdout.is_empty() {
            reason.stdout
        } else {
            "unknown".into()
        };

        let restored_files = if let Some(fp) = file_path {
            vec![fp.to_string()]
        } else {
            list_worktree_files(&abs, &store, commit_hash)?
        };

        if let Some(tool_ctx) = ctx {
            for rel in &restored_files {
                tool_ctx.record_mutation(MutationRecord {
                    path: rel.clone(),
                    kind: MutationKind::Modify,
                    lines_added: 0,
                    lines_removed: 0,
                });
            }
        }

        Ok(RestoreResult {
            restored_to: short.to_string(),
            reason: reason_text,
            files_restored: restored_files.len(),
            restored_files,
        })
    }

    pub fn save(&mut self, working_dir: &Path, reason: &str) -> Result<bool, String> {
        self.take(working_dir, reason)
    }

    pub fn diff_against_n(&self, working_dir: &Path, n: usize) -> Result<String, String> {
        let hash = self
            .list_checkpoints(working_dir)
            .into_iter()
            .find(|e| e.n == n)
            .map(|e| e.hash)
            .ok_or_else(|| format!("Checkpoint #{n} not found"))?;

        let base = checkpoint_base(&self.cfg.edgecrab_home);
        let store = store_path(&base);
        let abs = normalize_path(working_dir);
        let index_file = index_path(&store, &project_hash(&abs));
        let reference = ref_name(&project_hash(&abs));

        let _ = run_git(
            &["add", "-A"],
            &store,
            &abs,
            Some(&index_file),
            &HashSet::new(),
            GIT_TIMEOUT_SECS * 2,
        );
        let stat = run_git(
            &["diff", "--stat", &hash, "--cached"],
            &store,
            &abs,
            Some(&index_file),
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        let diff = run_git(
            &["diff", &hash, "--cached", "--no-color"],
            &store,
            &abs,
            Some(&index_file),
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        let _ = run_git(
            &["read-tree", &reference],
            &store,
            &abs,
            Some(&index_file),
            &HashSet::from([128i32]),
            GIT_TIMEOUT_SECS,
        );

        if stat.stdout.is_empty() && diff.stdout.is_empty() {
            return Ok("no changes vs current state".into());
        }
        Ok(format!(
            "{}{}",
            if stat.stdout.is_empty() {
                String::new()
            } else {
                format!("{}\n\n", stat.stdout)
            },
            diff.stdout
        ))
    }
}
