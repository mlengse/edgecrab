//! Checkpoint store maintenance (`edgecrab checkpoints` CLI).

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::display::format_bytes;
use super::git::{
    checkpoint_base, dir_size_bytes, ref_name, run_git, store_path, GIT_TIMEOUT_SECS,
    LEGACY_PREFIX, PROJECTS_DIRNAME,
};

#[derive(Debug, Clone, Serialize)]
pub struct StoreProject {
    pub hash: String,
    pub workdir: String,
    pub exists: bool,
    pub commits: u32,
    pub last_touch: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegacyArchive {
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoreStatus {
    pub base: PathBuf,
    pub store_size_bytes: u64,
    pub legacy_size_bytes: u64,
    pub total_size_bytes: u64,
    pub project_count: usize,
    pub projects: Vec<StoreProject>,
    pub legacy_archives: Vec<LegacyArchive>,
}

pub fn store_status(edgecrab_home: &Path) -> StoreStatus {
    let base = checkpoint_base(edgecrab_home);
    let mut out = StoreStatus {
        base: base.clone(),
        store_size_bytes: 0,
        legacy_size_bytes: 0,
        total_size_bytes: 0,
        project_count: 0,
        projects: Vec::new(),
        legacy_archives: Vec::new(),
    };
    if !base.exists() {
        return out;
    }

    let store = store_path(&base);
    if store.exists() {
        out.store_size_bytes = dir_size_bytes(&store);
        let projects_dir = store.join(PROJECTS_DIRNAME);
        if projects_dir.exists() {
            for entry in std::fs::read_dir(&projects_dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let hash = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let meta = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
                let workdir = meta
                    .as_ref()
                    .and_then(|m| m.get("workdir"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let last_touch = meta
                    .as_ref()
                    .and_then(|m| m.get("last_touch"))
                    .and_then(|v| v.as_f64());
                let reference = ref_name(&hash);
                let count = run_git(
                    &["rev-list", "--count", &reference],
                    &store,
                    &base,
                    None,
                    &std::collections::HashSet::from([128i32]),
                    GIT_TIMEOUT_SECS,
                );
                let commits = count.stdout.parse().unwrap_or(0);
                out.projects.push(StoreProject {
                    hash,
                    workdir: workdir.clone(),
                    exists: !workdir.is_empty() && Path::new(&workdir).exists(),
                    commits,
                    last_touch,
                });
            }
        }
    }
    out.project_count = out.projects.len();

    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() && name.starts_with(LEGACY_PREFIX) {
                let size = dir_size_bytes(&entry.path());
                out.legacy_size_bytes = out.legacy_size_bytes.saturating_add(size);
                out.legacy_archives.push(LegacyArchive {
                    name,
                    size_bytes: size,
                });
            }
        }
    }

    out.total_size_bytes = dir_size_bytes(&base);
    out
}

pub fn format_store_status(status: &StoreStatus, limit: usize) -> String {
    let mut lines = vec![
        format!("Checkpoint base: {}", status.base.display()),
        format!("Total size:      {}", format_bytes(status.total_size_bytes)),
        format!("  store/         {}", format_bytes(status.store_size_bytes)),
        format!(
            "  legacy-*       {}",
            format_bytes(status.legacy_size_bytes)
        ),
        format!("Projects:        {}", status.project_count),
    ];
    if !status.projects.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "  {:<60}  {:>7}  {:>12}  STATE",
            "WORKDIR", "COMMITS", "LAST TOUCH"
        ));
        let mut sorted = status.projects.clone();
        sorted.sort_by(|a, b| {
            b.last_touch
                .unwrap_or(0.0)
                .partial_cmp(&a.last_touch.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for p in sorted.into_iter().take(limit) {
            let mut wd = p.workdir.clone();
            if wd.len() > 60 {
                wd = format!("…{}", &wd[wd.len().saturating_sub(59)..]);
            }
            let state = if p.exists { "live" } else { "orphan" };
            let last = p
                .last_touch
                .map(format_age)
                .unwrap_or_else(|| "—".into());
            lines.push(format!(
                "  {wd:<60}  {:>7}  {:>12}  {state}",
                p.commits, last
            ));
        }
    }
    if !status.legacy_archives.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Legacy archives ({}):",
            status.legacy_archives.len()
        ));
        for arch in &status.legacy_archives {
            lines.push(format!(
                "  {:<40}  {:>10}",
                arch.name,
                format_bytes(arch.size_bytes)
            ));
        }
        lines.push(String::new());
        lines.push("Clear with: edgecrab checkpoints clear-legacy".into());
    }
    lines.join("\n")
}

fn format_age(ts: f64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(ts);
    let age = now - ts;
    if age < 60.0 {
        format!("{}s ago", age as u64)
    } else if age < 3600.0 {
        format!("{}m ago", (age / 60.0) as u64)
    } else if age < 86400.0 {
        format!("{}h ago", (age / 3600.0) as u64)
    } else {
        format!("{}d ago", (age / 86400.0) as u64)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ClearResult {
    pub bytes_freed: u64,
    pub deleted: bool,
}

pub fn clear_all(edgecrab_home: &Path) -> ClearResult {
    let base = checkpoint_base(edgecrab_home);
    let mut out = ClearResult {
        bytes_freed: 0,
        deleted: false,
    };
    if !base.exists() {
        return out;
    }
    let size = dir_size_bytes(&base);
    if std::fs::remove_dir_all(&base).is_ok() {
        out.bytes_freed = size;
        out.deleted = true;
    }
    out
}

#[derive(Debug, Clone, Serialize)]
pub struct ClearLegacyResult {
    pub bytes_freed: u64,
    pub deleted: u32,
}

pub fn clear_legacy(edgecrab_home: &Path) -> ClearLegacyResult {
    let base = checkpoint_base(edgecrab_home);
    let mut out = ClearLegacyResult {
        bytes_freed: 0,
        deleted: 0,
    };
    if !base.exists() {
        return out;
    }
    for entry in std::fs::read_dir(&base).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() && name.starts_with(LEGACY_PREFIX) {
            let size = dir_size_bytes(&entry.path());
            if std::fs::remove_dir_all(entry.path()).is_ok() {
                out.bytes_freed = out.bytes_freed.saturating_add(size);
                out.deleted += 1;
            }
        }
    }
    out
}
