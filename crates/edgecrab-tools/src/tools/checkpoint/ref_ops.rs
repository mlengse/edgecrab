//! Git ref maintenance: gc, chain rebuild, size estimation, legacy migration.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::git::{
    GIT_TIMEOUT_SECS, LEGACY_PREFIX, PRUNE_MARKER_NAME, REFS_PREFIX, STORE_DIRNAME, dir_size_bytes,
    run_git, store_path,
};
use super::types::CheckpointEntry;

pub fn gc_store(store: &Path, working_dir: &Path) -> Result<(), String> {
    let _ = run_git(
        &["reflog", "expire", "--expire=now", "--all"],
        store,
        working_dir,
        None,
        &HashSet::new(),
        GIT_TIMEOUT_SECS,
    );
    let _ = run_git(
        &["gc", "--prune=now", "--quiet"],
        store,
        working_dir,
        None,
        &HashSet::new(),
        GIT_TIMEOUT_SECS * 3,
    );
    Ok(())
}

pub fn rebuild_ref_chain(
    store: &Path,
    working_dir: &Path,
    commits: &[&str],
) -> Result<Option<String>, String> {
    let mut new_parent: Option<String> = None;
    for sha in commits {
        let tree = run_git(
            &["rev-parse", &format!("{sha}^{{tree}}")],
            store,
            working_dir,
            None,
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        if !tree.ok || tree.stdout.is_empty() {
            return Ok(None);
        }
        let msg = run_git(
            &["log", "--format=%s", "-1", sha],
            store,
            working_dir,
            None,
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        let commit_msg = if msg.ok && !msg.stdout.is_empty() {
            msg.stdout.as_str()
        } else {
            "checkpoint"
        };
        let commit_args: Vec<&str> = if let Some(parent) = new_parent.as_deref() {
            vec![
                "commit-tree",
                &tree.stdout,
                "-p",
                parent,
                "-m",
                commit_msg,
                "--no-gpg-sign",
            ]
        } else {
            vec![
                "commit-tree",
                &tree.stdout,
                "-m",
                commit_msg,
                "--no-gpg-sign",
            ]
        };
        let commit = run_git(
            &commit_args,
            store,
            working_dir,
            None,
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        if !commit.ok || commit.stdout.is_empty() {
            return Ok(None);
        }
        new_parent = Some(commit.stdout);
    }
    Ok(new_parent)
}

pub fn drop_oldest_commit(store: &Path, base: &Path, reference: &str) -> Result<bool, String> {
    let allowed = HashSet::from([128i32]);
    let count_out = run_git(
        &["rev-list", "--count", reference],
        store,
        base,
        None,
        &allowed,
        GIT_TIMEOUT_SECS,
    );
    let count: usize = count_out.stdout.parse().unwrap_or(0);
    if count <= 1 {
        return Ok(false);
    }
    let list = run_git(
        &["rev-list", "--reverse", reference],
        store,
        base,
        None,
        &allowed,
        GIT_TIMEOUT_SECS,
    );
    if !list.ok || list.stdout.is_empty() {
        return Ok(false);
    }
    let commits: Vec<&str> = list.stdout.lines().collect();
    let keep = &commits[1..];
    if let Some(new_tip) = rebuild_ref_chain(store, base, keep)? {
        let _ = run_git(
            &["update-ref", reference, &new_tip],
            store,
            base,
            None,
            &HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        return Ok(true);
    }
    Ok(false)
}

pub fn parse_shortstat(line: &str, entry: &mut CheckpointEntry) {
    if let Ok(re) = regex::Regex::new(r"(\d+) file")
        && let Some(m) = re.captures(line)
    {
        entry.files_changed = m.get(1).and_then(|c| c.as_str().parse().ok()).unwrap_or(0);
    }
    if let Ok(re) = regex::Regex::new(r"(\d+) insertion")
        && let Some(m) = re.captures(line)
    {
        entry.insertions = m.get(1).and_then(|c| c.as_str().parse().ok()).unwrap_or(0);
    }
    if let Ok(re) = regex::Regex::new(r"(\d+) deletion")
        && let Some(m) = re.captures(line)
    {
        entry.deletions = m.get(1).and_then(|c| c.as_str().parse().ok()).unwrap_or(0);
    }
}

pub fn estimate_commit_bytes(store: &Path, working_dir: &Path, commit: &str) -> u64 {
    let out = run_git(
        &["rev-list", "--objects", commit],
        store,
        working_dir,
        None,
        &HashSet::from([128i32]),
        GIT_TIMEOUT_SECS,
    );
    if !out.ok {
        return 0;
    }
    let mut total = 0u64;
    for line in out.stdout.lines() {
        let sha = line.split_whitespace().next().unwrap_or("");
        if sha.len() >= 4 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
            let sz = run_git(
                &["cat-file", "-s", sha],
                store,
                working_dir,
                None,
                &HashSet::new(),
                GIT_TIMEOUT_SECS,
            );
            if sz.ok {
                total = total.saturating_add(sz.stdout.parse().unwrap_or(0));
            }
        }
    }
    total
}

pub fn drop_oversize_from_index(store: &Path, working_dir: &Path, index_file: &Path, max_mb: u32) {
    use super::git::normalize_path;

    let cap = u64::from(max_mb) * 1024 * 1024;
    let ls = run_git(
        &["ls-files", "--cached", "-z"],
        store,
        working_dir,
        Some(index_file),
        &HashSet::new(),
        GIT_TIMEOUT_SECS,
    );
    if !ls.ok || ls.stdout.is_empty() {
        return;
    }
    let abs = normalize_path(working_dir);
    let mut oversize = Vec::new();
    for rel in ls.stdout.split('\0').filter(|s| !s.is_empty()) {
        if let Ok(meta) = abs.join(rel).metadata()
            && meta.len() > cap
        {
            oversize.push(rel.to_string());
        }
    }
    for chunk in oversize.chunks(200) {
        let mut args = vec!["rm", "--cached", "--quiet", "--"];
        for p in chunk {
            args.push(p.as_str());
        }
        let _ = run_git(
            &args,
            store,
            working_dir,
            Some(index_file),
            &HashSet::from([128i32]),
            GIT_TIMEOUT_SECS,
        );
    }
}

pub fn list_worktree_files(
    working_dir: &Path,
    store: &Path,
    commit: &str,
) -> Result<Vec<String>, String> {
    let out = run_git(
        &["ls-tree", "-r", "--name-only", commit],
        store,
        working_dir,
        None,
        &HashSet::new(),
        GIT_TIMEOUT_SECS,
    );
    if !out.ok {
        return Err(out.stderr);
    }
    Ok(out
        .stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn migrate_legacy_store(base: &Path) -> Result<(), String> {
    if !base.exists() {
        let _ = std::fs::create_dir_all(base);
        return Ok(());
    }
    let store = store_path(base);
    if store.join("HEAD").exists() {
        return Ok(());
    }
    let reserved: HashSet<&str> = HashSet::from([STORE_DIRNAME, PRUNE_MARKER_NAME]);
    let mut legacy_root: Option<PathBuf> = None;
    for entry in std::fs::read_dir(base).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if reserved.contains(name.as_str()) || name.starts_with(LEGACY_PREFIX) {
            continue;
        }
        let root = legacy_root.get_or_insert_with(|| {
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
            let path = base.join(format!("{LEGACY_PREFIX}{stamp}"));
            let _ = std::fs::create_dir_all(&path);
            path
        });
        let dest = root.join(&name);
        let _ = std::fs::rename(entry.path(), dest);
    }
    Ok(())
}

pub fn enforce_size_cap(store: &Path, base: &Path, max_total_size_mb: u32) -> Result<(), String> {
    if max_total_size_mb == 0 {
        return Ok(());
    }
    let cap = u64::from(max_total_size_mb) * 1024 * 1024;

    for _ in 0..20 {
        if dir_size_bytes(store) <= cap {
            break;
        }
        let refs = run_git(
            &["for-each-ref", "--format=%(refname)", REFS_PREFIX],
            store,
            base,
            None,
            &HashSet::from([128i32]),
            GIT_TIMEOUT_SECS,
        );
        if !refs.ok || refs.stdout.is_empty() {
            break;
        }
        let mut dropped = false;
        for reference in refs.stdout.lines() {
            if drop_oldest_commit(store, base, reference)? {
                dropped = true;
            }
        }
        if !dropped {
            break;
        }
    }
    gc_store(store, base)
}
