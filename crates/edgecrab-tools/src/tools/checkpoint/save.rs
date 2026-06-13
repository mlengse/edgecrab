//! Checkpoint save (git commit-tree) and per-ref FIFO pruning.

use std::collections::{HashSet, VecDeque};
use std::path::Path;

use super::git::{
    GIT_TIMEOUT_SECS, checkpoint_base, dir_file_count, index_path, init_store, load_pinned_shas,
    normalize_path, project_hash, ref_name, run_git, store_path, touch_project,
};
use super::manager::CheckpointManager;
use super::ref_ops::{
    drop_oversize_from_index, enforce_size_cap, gc_store, migrate_legacy_store, rebuild_ref_chain,
};

const MAX_TRACKED_FILES: usize = 50_000;

impl CheckpointManager {
    pub(crate) fn take(&mut self, working_dir: &Path, reason: &str) -> Result<bool, String> {
        let base = checkpoint_base(&self.cfg.edgecrab_home);
        migrate_legacy_store(&base)?;
        let store = store_path(&base);
        if let Some(err) = init_store(&store, &base, working_dir) {
            return Err(err);
        }
        touch_project(&store, working_dir);

        if dir_file_count(working_dir) > MAX_TRACKED_FILES {
            return Ok(false);
        }

        let abs = normalize_path(working_dir);
        let dir_hash = project_hash(&abs);
        let index_file = index_path(&store, &dir_hash);
        let reference = ref_name(&dir_hash);
        let allowed_quiet = HashSet::from([1i32]);
        let allowed_ref = HashSet::from([128i32]);

        if index_file.exists() {
            let ref_commit = run_git(
                &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
                &store,
                &abs,
                None,
                &allowed_ref,
                GIT_TIMEOUT_SECS,
            );
            if ref_commit.ok && !ref_commit.stdout.is_empty() {
                let _ = run_git(
                    &["read-tree", &ref_commit.stdout],
                    &store,
                    &abs,
                    Some(&index_file),
                    &allowed_ref,
                    GIT_TIMEOUT_SECS,
                );
            } else {
                let _ = std::fs::remove_file(&index_file);
            }
        } else if let Some(parent) = index_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let add = run_git(
            &["add", "-A"],
            &store,
            &abs,
            Some(&index_file),
            &HashSet::new(),
            GIT_TIMEOUT_SECS * 2,
        );
        if !add.ok {
            return Err(format!("git add failed: {}", add.stderr));
        }

        if self.cfg.max_file_size_mb > 0 {
            drop_oversize_from_index(&store, &abs, &index_file, self.cfg.max_file_size_mb);
        }

        let ref_commit = run_git(
            &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
            &store,
            &abs,
            None,
            &allowed_ref,
            GIT_TIMEOUT_SECS,
        );
        let has_ref = ref_commit.ok && !ref_commit.stdout.is_empty();

        if has_ref {
            let diff = run_git(
                &["diff-index", "--cached", "--quiet", &ref_commit.stdout],
                &store,
                &abs,
                Some(&index_file),
                &allowed_quiet,
                GIT_TIMEOUT_SECS,
            );
            if diff.ok {
                return Ok(false);
            }
        } else {
            let ls = run_git(
                &["ls-files", "--cached"],
                &store,
                &abs,
                Some(&index_file),
                &HashSet::new(),
                GIT_TIMEOUT_SECS,
            );
            if ls.ok && ls.stdout.trim().is_empty() {
                return Ok(false);
            }
        }

        let tree = run_git(
            &["write-tree"],
            &store,
            &abs,
            Some(&index_file),
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        if !tree.ok || tree.stdout.is_empty() {
            return Err(format!("write-tree failed: {}", tree.stderr));
        }

        let commit_args: Vec<&str> = if has_ref {
            vec![
                "commit-tree",
                &tree.stdout,
                "-p",
                &ref_commit.stdout,
                "-m",
                reason,
                "--no-gpg-sign",
            ]
        } else {
            vec!["commit-tree", &tree.stdout, "-m", reason, "--no-gpg-sign"]
        };
        let commit = run_git(
            &commit_args,
            &store,
            &abs,
            Some(&index_file),
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        if !commit.ok || commit.stdout.is_empty() {
            return Err(format!("commit-tree failed: {}", commit.stderr));
        }

        let update_args: Vec<&str> = if has_ref {
            vec!["update-ref", &reference, &commit.stdout, &ref_commit.stdout]
        } else {
            vec!["update-ref", &reference, &commit.stdout]
        };
        let update = run_git(
            &update_args,
            &store,
            &abs,
            None,
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        if !update.ok {
            return Err(format!("update-ref failed: {}", update.stderr));
        }

        self.prune_ref(&store, &abs, &reference)?;
        enforce_size_cap(&store, &base, self.cfg.max_total_size_mb)?;
        Ok(true)
    }

    fn prune_ref(&self, store: &Path, working_dir: &Path, reference: &str) -> Result<(), String> {
        let allowed = HashSet::from([128i32]);
        let count_out = run_git(
            &["rev-list", "--count", reference],
            store,
            working_dir,
            None,
            &allowed,
            GIT_TIMEOUT_SECS,
        );
        if !count_out.ok {
            return Ok(());
        }
        let count: usize = count_out.stdout.parse().unwrap_or(0);
        let max = self.cfg.max_snapshots as usize;
        if count <= max {
            return Ok(());
        }

        let list = run_git(
            &["rev-list", "--reverse", reference],
            store,
            working_dir,
            None,
            &allowed,
            GIT_TIMEOUT_SECS,
        );
        if !list.ok || list.stdout.is_empty() {
            return Ok(());
        }

        let commits: Vec<&str> = list.stdout.lines().collect();
        let pinned = load_pinned_shas(store, working_dir);

        let mut keep: VecDeque<&str> = commits.iter().copied().collect();
        while keep.len() > max {
            if let Some(pos) = keep.iter().position(|sha| !pinned.contains(*sha)) {
                keep.remove(pos);
            } else {
                break;
            }
        }

        let ordered: Vec<&str> = keep.into();
        if let Some(new_tip) = rebuild_ref_chain(store, working_dir, &ordered)? {
            let _ = run_git(
                &["update-ref", reference, &new_tip],
                store,
                working_dir,
                None,
                &HashSet::new(),
                GIT_TIMEOUT_SECS,
            );
            gc_store(store, working_dir)?;
        }
        Ok(())
    }
}
