//! Startup auto-maintenance for the checkpoint store (Hermes `prv_auto_prune_checkpoints`).

use std::path::Path;

use tracing::info;

use super::git::{
    GIT_TIMEOUT_SECS, LEGACY_PREFIX, PRUNE_MARKER_NAME, REFS_PREFIX, STORE_DIRNAME,
    checkpoint_base, dir_size_bytes, index_path, ref_name, run_git, store_path,
};
use super::ref_ops::{gc_store, rebuild_ref_chain};

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PruneCounts {
    pub scanned: u32,
    pub deleted_orphan: u32,
    pub deleted_stale: u32,
    pub errors: u32,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AutoPruneResult {
    pub skipped: bool,
    pub result: PruneCounts,
    pub error: Option<String>,
}

/// Delete stale/orphan checkpoint projects and reclaim store space.
pub fn prune_checkpoints(
    edgecrab_home: &Path,
    retention_days: u32,
    delete_orphans: bool,
    max_total_size_mb: u32,
) -> PruneCounts {
    let mut result = PruneCounts::default();
    let base = checkpoint_base(edgecrab_home);
    if !base.exists() {
        return result;
    }

    let size_before = dir_size_bytes(&base);
    let cutoff = if retention_days > 0 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64() - f64::from(retention_days) * 86400.0)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    // Legacy pre-v2 per-project shadow repos
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == STORE_DIRNAME {
                continue;
            }
            if name.starts_with(LEGACY_PREFIX) && retention_days > 0 {
                if let Ok(meta) = path.metadata() {
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs_f64())
                        .unwrap_or(0.0);
                    if mtime < cutoff {
                        let size = dir_size_bytes(&path);
                        if std::fs::remove_dir_all(&path).is_ok() {
                            result.bytes_freed = result.bytes_freed.saturating_add(size);
                            result.deleted_stale += 1;
                        } else {
                            result.errors += 1;
                        }
                    }
                }
                continue;
            }
            if !(path.join("HEAD")).exists() {
                continue;
            }
            result.scanned += 1;
            let mut reason: Option<&str> = None;
            if delete_orphans {
                let marker = path.join("EDGECRAB_WORKDIR");
                let workdir = std::fs::read_to_string(&marker)
                    .ok()
                    .map(|s| s.trim().to_string());
                if workdir.as_ref().is_none_or(|w| !Path::new(w).exists()) {
                    reason = Some("orphan");
                }
            }
            if reason.is_none() && retention_days > 0 {
                let newest = dir_newest_mtime(&path);
                if newest > 0.0 && newest < cutoff {
                    reason = Some("stale");
                }
            }
            if let Some(r) = reason {
                let size = dir_size_bytes(&path);
                if std::fs::remove_dir_all(&path).is_ok() {
                    result.bytes_freed = result.bytes_freed.saturating_add(size);
                    if r == "orphan" {
                        result.deleted_orphan += 1;
                    } else {
                        result.deleted_stale += 1;
                    }
                } else {
                    result.errors += 1;
                }
            }
        }
    }

    // v2 shared store
    let store = store_path(&base);
    if store.join("HEAD").exists() {
        let projects_dir = store.join("projects");
        if projects_dir.exists() {
            for entry in std::fs::read_dir(&projects_dir)
                .into_iter()
                .flatten()
                .flatten()
            {
                let meta_path = entry.path();
                if meta_path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let dir_hash = meta_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let Ok(text) = std::fs::read_to_string(&meta_path) else {
                    continue;
                };
                let Ok(meta) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                result.scanned += 1;
                let workdir = meta.get("workdir").and_then(|v| v.as_str()).unwrap_or("");
                let mut reason: Option<&str> = None;
                if delete_orphans && (workdir.is_empty() || !Path::new(workdir).exists()) {
                    reason = Some("orphan");
                } else if retention_days > 0 {
                    let last_touch = meta
                        .get("last_touch")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    if last_touch > 0.0 && last_touch < cutoff {
                        reason = Some("stale");
                    }
                }
                if reason.is_none() {
                    continue;
                }
                let reference = ref_name(dir_hash);
                let _ = run_git(
                    &["update-ref", "-d", &reference],
                    &store,
                    &base,
                    None,
                    &std::collections::HashSet::from([128i32]),
                    GIT_TIMEOUT_SECS,
                );
                let _ = std::fs::remove_file(index_path(&store, dir_hash));
                let _ = std::fs::remove_file(&meta_path);
                if reason == Some("orphan") {
                    result.deleted_orphan += 1;
                } else {
                    result.deleted_stale += 1;
                }
            }
        }
        let _ = gc_store(&store, &base);

        if max_total_size_mb > 0 {
            enforce_global_size_cap(&store, &base, max_total_size_mb);
        }
    }

    let size_after = dir_size_bytes(&base);
    result.bytes_freed = result
        .bytes_freed
        .max(size_before.saturating_sub(size_after));
    result
}

pub fn maybe_auto_prune_checkpoints(
    edgecrab_home: &Path,
    retention_days: u32,
    min_interval_hours: u32,
    delete_orphans: bool,
    max_total_size_mb: u32,
) -> AutoPruneResult {
    let base = checkpoint_base(edgecrab_home);
    if !base.exists() {
        return AutoPruneResult {
            skipped: false,
            result: PruneCounts::default(),
            error: None,
        };
    }

    let marker = base.join(PRUNE_MARKER_NAME);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    if marker.exists()
        && let Ok(text) = std::fs::read_to_string(&marker)
        && let Ok(last) = text.trim().parse::<f64>()
        && now - last < f64::from(min_interval_hours) * 3600.0
    {
        return AutoPruneResult {
            skipped: true,
            result: PruneCounts::default(),
            error: None,
        };
    }

    let result = prune_checkpoints(
        edgecrab_home,
        retention_days,
        delete_orphans,
        max_total_size_mb,
    );
    let _ = std::fs::write(&marker, now.to_string());

    let total = result.deleted_orphan + result.deleted_stale;
    if total > 0 {
        info!(
            "checkpoint auto-maintenance: pruned {total} entries ({} orphan, {} stale), reclaimed {:.1} MB",
            result.deleted_orphan,
            result.deleted_stale,
            result.bytes_freed as f64 / (1024.0 * 1024.0)
        );
    }

    AutoPruneResult {
        skipped: false,
        result,
        error: None,
    }
}

fn dir_newest_mtime(path: &Path) -> f64 {
    let mut newest = 0.0f64;
    fn walk(dir: &Path, newest: &mut f64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata()
                && let Ok(t) = meta.modified()
                && let Ok(d) = t.duration_since(std::time::UNIX_EPOCH)
            {
                *newest = newest.max(d.as_secs_f64());
            }
            let p = entry.path();
            if p.is_dir() {
                walk(&p, newest);
            }
        }
    }
    walk(path, &mut newest);
    newest
}

fn enforce_global_size_cap(store: &Path, base: &Path, max_mb: u32) {
    let cap = u64::from(max_mb) * 1024 * 1024;
    for _ in 0..20 {
        if dir_size_bytes(store) <= cap {
            break;
        }
        let refs = run_git(
            &["for-each-ref", "--format=%(refname)", REFS_PREFIX],
            store,
            base,
            None,
            &std::collections::HashSet::from([128i32]),
            GIT_TIMEOUT_SECS,
        );
        if !refs.ok || refs.stdout.is_empty() {
            break;
        }
        let mut dropped = false;
        for reference in refs.stdout.lines() {
            if drop_oldest_commit_prune(store, base, reference) {
                dropped = true;
            }
        }
        if !dropped {
            break;
        }
    }
    let _ = gc_store(store, base);
}

fn drop_oldest_commit_prune(store: &Path, base: &Path, reference: &str) -> bool {
    let allowed = std::collections::HashSet::from([128i32]);
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
        return false;
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
        return false;
    }
    let commits: Vec<&str> = list.stdout.lines().collect();
    let keep = &commits[1..];
    if let Ok(Some(new_tip)) = rebuild_ref_chain(store, base, keep) {
        let _ = run_git(
            &["update-ref", reference, &new_tip],
            store,
            base,
            None,
            &std::collections::HashSet::new(),
            GIT_TIMEOUT_SECS,
        );
        return true;
    }
    false
}
