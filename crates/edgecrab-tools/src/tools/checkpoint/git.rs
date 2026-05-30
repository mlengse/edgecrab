//! Git subprocess helpers for the shared checkpoint shadow store.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use regex::Regex;
use sha2::{Digest, Sha256};

use super::excludes::default_exclude_file_content;

pub const STORE_DIRNAME: &str = "store";
pub const REFS_PREFIX: &str = "refs/edgecrab";
pub const INDEXES_DIRNAME: &str = "indexes";
pub const PROJECTS_DIRNAME: &str = "projects";
pub const LEGACY_PREFIX: &str = "legacy-";
pub const PRUNE_MARKER_NAME: &str = ".last_prune";

fn commit_hash_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9a-fA-F]{4,64}$").expect("valid commit hash regex"))
}
pub const GIT_TIMEOUT_SECS: u64 = 30;
const MAX_FILES: usize = 50_000;

/// Deterministic per-project hash: sha256(abs_path)[:16].
pub fn project_hash(working_dir: &Path) -> String {
    let abs = normalize_path(working_dir);
    let mut h = Sha256::new();
    h.update(abs.to_string_lossy().as_bytes());
    format!("{:x}", h.finalize())[..16].to_string()
}

/// Canonical absolute path for checkpoint operations.
pub fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn checkpoint_base(edgecrab_home: &Path) -> PathBuf {
    edgecrab_home.join("checkpoints")
}

pub fn store_path(base: &Path) -> PathBuf {
    base.join(STORE_DIRNAME)
}

pub fn index_path(store: &Path, dir_hash: &str) -> PathBuf {
    store.join(INDEXES_DIRNAME).join(dir_hash)
}

pub fn ref_name(dir_hash: &str) -> String {
    format!("{REFS_PREFIX}/{dir_hash}")
}

pub fn project_meta_path(store: &Path, dir_hash: &str) -> PathBuf {
    store
        .join(PROJECTS_DIRNAME)
        .join(format!("{dir_hash}.json"))
}

pub fn validate_commit_hash(commit_hash: &str) -> Option<String> {
    let trimmed = commit_hash.trim();
    if trimmed.is_empty() {
        return Some("Empty commit hash".into());
    }
    if trimmed.starts_with('-') {
        return Some(format!(
            "Invalid commit hash (must not start with '-'): {trimmed:?}"
        ));
    }
    if !commit_hash_re().is_match(trimmed) {
        return Some(format!(
            "Invalid commit hash (expected 4-64 hex characters): {trimmed:?}"
        ));
    }
    None
}

pub fn validate_file_path(file_path: &str, working_dir: &Path) -> Option<String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Some("Empty file path".into());
    }
    if Path::new(trimmed).is_absolute() {
        return Some(format!(
            "File path must be relative, got absolute path: {trimmed:?}"
        ));
    }
    if trimmed.split('/').any(|c| c == "..") {
        return Some(format!(
            "File path escapes the working directory via traversal: {trimmed:?}"
        ));
    }
    let abs_workdir = normalize_path(working_dir);
    let resolved = abs_workdir.join(trimmed);
    if !resolved.starts_with(&abs_workdir) {
        return Some(format!(
            "File path escapes the working directory via traversal: {trimmed:?}"
        ));
    }
    None
}

struct GitEnv {
    env: Vec<(String, String)>,
    cwd: PathBuf,
}

fn git_env(store: &Path, working_dir: &Path, index_file: Option<&Path>) -> GitEnv {
    let normalized = normalize_path(working_dir);
    let mut pairs = Vec::new();
    for (k, v) in std::env::vars() {
        if k.starts_with("GIT_") {
            continue;
        }
        pairs.push((k, v));
    }
    pairs.push(("GIT_DIR".into(), store.to_string_lossy().into_owned()));
    pairs.push((
        "GIT_WORK_TREE".into(),
        normalized.to_string_lossy().into_owned(),
    ));
    if let Some(idx) = index_file {
        pairs.push(("GIT_INDEX_FILE".into(), idx.to_string_lossy().into_owned()));
    }
    pairs.push(("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()));
    pairs.push(("GIT_CONFIG_SYSTEM".into(), "/dev/null".into()));
    pairs.push(("GIT_CONFIG_NOSYSTEM".into(), "1".into()));
    GitEnv {
        env: pairs,
        cwd: normalized,
    }
}

fn apply_env(cmd: &mut Command, git: &GitEnv) {
    cmd.env_clear();
    for (k, v) in &git.env {
        cmd.env(k, v);
    }
    cmd.current_dir(&git.cwd);
}

pub struct GitResult {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_git(
    args: &[&str],
    store: &Path,
    working_dir: &Path,
    index_file: Option<&Path>,
    allowed_returncodes: &HashSet<i32>,
    _timeout_secs: u64,
) -> GitResult {
    let wd = normalize_path(working_dir);
    if !wd.is_dir() {
        return GitResult {
            ok: false,
            stdout: String::new(),
            stderr: format!("working directory is not a directory: {}", wd.display()),
        };
    }

    let git = git_env(store, &wd, index_file);
    let mut cmd = Command::new("git");
    cmd.args(args);
    apply_env(&mut cmd, &git);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return GitResult {
                ok: false,
                stdout: String::new(),
                stderr: e.to_string(),
            };
        }
    };

    parse_git_output(output, allowed_returncodes)
}

fn parse_git_output(output: Output, allowed_returncodes: &HashSet<i32>) -> GitResult {
    let rc = output.status.code().unwrap_or(-1);
    let ok = output.status.success() || allowed_returncodes.contains(&rc);
    GitResult {
        ok,
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    }
}

pub fn run_git_init_bare(store: &Path) -> Result<(), String> {
    std::fs::create_dir_all(store).map_err(|e| e.to_string())?;
    let mut cmd = Command::new("git");
    cmd.args(["init", "--bare"]).arg(store);
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
    cmd.env("GIT_CONFIG_SYSTEM", "/dev/null");
    cmd.env("GIT_CONFIG_NOSYSTEM", "1");
    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

pub fn dir_file_count(path: &Path) -> usize {
    fn walk(dir: &Path, count: &mut usize) {
        if *count > MAX_FILES {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            *count += 1;
            if *count > MAX_FILES {
                return;
            }
            let p = entry.path();
            if p.is_dir() {
                walk(&p, count);
            }
        }
    }
    let mut count = 0usize;
    walk(path, &mut count);
    count
}

pub fn dir_size_bytes(path: &Path) -> u64 {
    fn walk(dir: &Path, total: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() {
                if let Ok(meta) = p.metadata() {
                    *total = total.saturating_add(meta.len());
                }
            } else if p.is_dir() {
                walk(&p, total);
            }
        }
    }
    let mut total = 0u64;
    walk(path, &mut total);
    total
}

pub fn init_store(store: &Path, base: &Path, working_dir: &Path) -> Option<String> {
    if !(store.join("HEAD")).exists() {
        if let Err(e) = run_git_init_bare(store) {
            return Some(format!("Shadow store init failed: {e}"));
        }
        let _ = std::fs::create_dir_all(store.join(INDEXES_DIRNAME));
        let _ = std::fs::create_dir_all(store.join(PROJECTS_DIRNAME));

        let cfg_wd = base;
        for (k, v) in [
            ("user.email", "edgecrab@local"),
            ("user.name", "EdgeCrab Checkpoint"),
            ("commit.gpgsign", "false"),
            ("tag.gpgSign", "false"),
            ("gc.auto", "0"),
        ] {
            let allowed = HashSet::new();
            let _ = run_git(
                &["config", k, v],
                store,
                cfg_wd,
                None,
                &allowed,
                GIT_TIMEOUT_SECS,
            );
        }

        let info_dir = store.join("info");
        let _ = std::fs::create_dir_all(&info_dir);
        let _ = std::fs::write(info_dir.join("exclude"), default_exclude_file_content());
    }

    register_project(store, working_dir);
    None
}

pub fn register_project(store: &Path, working_dir: &Path) {
    let dir_hash = project_hash(working_dir);
    let meta_path = project_meta_path(store, &dir_hash);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let mut meta = serde_json::json!({
        "workdir": normalize_path(working_dir).to_string_lossy(),
        "created_at": now,
        "last_touch": now,
        "pinned": [],
    });

    if meta_path.exists()
        && let Ok(text) = std::fs::read_to_string(&meta_path)
        && let Ok(existing) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(created) = existing.get("created_at")
    {
        meta["created_at"] = created.clone();
        if let Some(pinned) = existing.get("pinned") {
            meta["pinned"] = pinned.clone();
        }
    }

    let _ = std::fs::create_dir_all(meta_path.parent().unwrap_or(store));
    let _ = std::fs::write(meta_path, meta.to_string());
}

pub fn touch_project(store: &Path, working_dir: &Path) {
    let dir_hash = project_hash(working_dir);
    let meta_path = project_meta_path(store, &dir_hash);
    if !meta_path.exists() {
        register_project(store, working_dir);
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let mut meta = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    if !meta.is_object() {
        meta = serde_json::json!({});
    }
    if let Some(obj) = meta.as_object_mut() {
        obj.insert(
            "workdir".into(),
            serde_json::Value::String(normalize_path(working_dir).to_string_lossy().into()),
        );
        obj.insert("last_touch".into(), serde_json::json!(now));
        obj.entry("created_at").or_insert(serde_json::json!(now));
    }
    let _ = std::fs::write(meta_path, meta.to_string());
}

pub fn load_pinned_shas(store: &Path, working_dir: &Path) -> HashSet<String> {
    let dir_hash = project_hash(working_dir);
    let meta_path = project_meta_path(store, &dir_hash);
    let Ok(text) = std::fs::read_to_string(meta_path) else {
        return HashSet::new();
    };
    let Ok(meta) = serde_json::from_str::<serde_json::Value>(&text) else {
        return HashSet::new();
    };
    meta.get("pinned")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn set_pin(
    store: &Path,
    working_dir: &Path,
    commit_hash: &str,
    pinned: bool,
) -> Result<(), String> {
    let dir_hash = project_hash(working_dir);
    let meta_path = project_meta_path(store, &dir_hash);
    let mut meta = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .unwrap_or_else(|| serde_json::json!({ "pinned": [] }));

    let pinned_arr = meta
        .as_object_mut()
        .and_then(|o| {
            o.entry("pinned")
                .or_insert(serde_json::json!([]))
                .as_array_mut()
        })
        .ok_or_else(|| "invalid metadata".to_string())?;

    if pinned {
        if !pinned_arr.iter().any(|v| v.as_str() == Some(commit_hash)) {
            pinned_arr.push(serde_json::Value::String(commit_hash.to_string()));
        }
    } else {
        pinned_arr.retain(|v| v.as_str() != Some(commit_hash));
    }
    std::fs::write(&meta_path, meta.to_string()).map_err(|e| e.to_string())
}
