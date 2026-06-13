//! Disk-backed spawn tree snapshots — Hermes `spawn_tree.save` / `list` / `load` parity.
//!
//! Layout: `~/.edgecrab/spawn-trees/<session_id>/<timestamp>.json`

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::TimeZone;
use serde::{Deserialize, Serialize};

use crate::spawn_history::{SpawnHistoryEntry, SpawnTurnSnapshot};

const SPAWN_TREE_INDEX: &str = "_index.jsonl";

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static TEST_SPAWN_TREES_ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SpawnTreeFile {
    session_id: String,
    started_at: Option<f64>,
    finished_at: f64,
    label: String,
    subagents: Vec<SpawnHistoryEntry>,
    #[serde(default)]
    token_est: u32,
    #[serde(default)]
    cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnTreeIndexEntry {
    pub path: PathBuf,
    pub session_id: String,
    pub finished_at: f64,
    pub started_at: Option<f64>,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpawnTreeIndexLine {
    path: PathBuf,
    session_id: String,
    finished_at: f64,
    #[serde(default)]
    started_at: Option<f64>,
    #[serde(default)]
    label: String,
    count: usize,
}

pub fn spawn_trees_root() -> PathBuf {
    #[cfg(test)]
    if let Some(root) = TEST_SPAWN_TREES_ROOT.with(|slot| slot.borrow().clone()) {
        return root;
    }
    edgecrab_core::edgecrab_home().join("spawn-trees")
}

fn sanitize_session_id(session_id: &str) -> String {
    let safe: String = session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "unknown".to_string()
    } else {
        safe
    }
}

pub fn session_dir(session_id: &str) -> PathBuf {
    let dir = spawn_trees_root().join(sanitize_session_id(session_id));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn snapshot_to_file(
    snapshot: &SpawnTurnSnapshot,
    session_id: &str,
    finished_at: f64,
) -> SpawnTreeFile {
    let duration = snapshot.total_duration_secs as f64;
    let started_at = if duration > 0.0 {
        Some((finished_at - duration).max(0.0))
    } else {
        None
    };
    SpawnTreeFile {
        session_id: session_id.to_string(),
        started_at,
        finished_at,
        label: snapshot.label.clone(),
        subagents: snapshot.delegates.clone(),
        token_est: snapshot.token_est,
        cost_usd: snapshot.cost_usd,
    }
}

fn file_to_snapshot(file: SpawnTreeFile) -> SpawnTurnSnapshot {
    SpawnTurnSnapshot::from_entries(
        file.label,
        file.subagents,
        crate::spawn_history::TurnCommitMetrics {
            token_est: file.token_est,
            cost_usd: file.cost_usd,
        },
    )
}

fn append_index(session_dir: &Path, entry: &SpawnTreeIndexEntry) {
    let line = SpawnTreeIndexLine {
        path: entry.path.clone(),
        session_id: entry.session_id.clone(),
        finished_at: entry.finished_at,
        started_at: entry.started_at,
        label: entry.label.clone(),
        count: entry.count,
    };
    let Ok(json) = serde_json::to_string(&line) else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(session_dir.join(SPAWN_TREE_INDEX))
    {
        let _ = writeln!(file, "{json}");
    }
}

fn read_index(session_dir: &Path) -> Vec<SpawnTreeIndexEntry> {
    let path = session_dir.join(SPAWN_TREE_INDEX);
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let row: SpawnTreeIndexLine = serde_json::from_str(&line).ok()?;
            if row.path.exists() {
                Some(SpawnTreeIndexEntry {
                    path: row.path,
                    session_id: row.session_id,
                    finished_at: row.finished_at,
                    started_at: row.started_at,
                    label: row.label,
                    count: row.count,
                })
            } else {
                None
            }
        })
        .collect()
}

fn scan_session_dir(session_dir: &Path) -> Vec<SpawnTreeIndexEntry> {
    let mut entries = Vec::new();
    let Ok(read) = fs::read_dir(session_dir) else {
        return entries;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some(SPAWN_TREE_INDEX) {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(file) = serde_json::from_str::<SpawnTreeFile>(&raw) else {
            continue;
        };
        let finished_at = file.finished_at;
        entries.push(SpawnTreeIndexEntry {
            path,
            session_id: file.session_id,
            finished_at,
            started_at: file.started_at,
            label: file.label,
            count: file.subagents.len(),
        });
    }
    entries.sort_by(|a, b| {
        b.finished_at
            .partial_cmp(&a.finished_at)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    entries
}

/// Resolve and validate a user-supplied path stays under `spawn-trees/`.
pub fn resolve_spawn_tree_path(raw: &str) -> Result<PathBuf, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("path required".into());
    }
    let root = spawn_trees_root()
        .canonicalize()
        .unwrap_or_else(|_| spawn_trees_root());
    let candidate = PathBuf::from(raw);
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    let resolved = resolved
        .canonicalize()
        .map_err(|e| format!("invalid path: {e}"))?;
    resolved
        .strip_prefix(&root)
        .map_err(|_| "path outside spawn-trees root".to_string())?;
    Ok(resolved)
}

/// Persist a completed turn snapshot (best-effort; errors are logged by caller).
pub fn save_turn_snapshot(
    session_id: &str,
    snapshot: &SpawnTurnSnapshot,
) -> Result<PathBuf, String> {
    if snapshot.delegates.is_empty() {
        return Err("empty snapshot".into());
    }
    let finished_at = unix_now();
    let dir = session_dir(session_id);
    let ts = chrono::Utc
        .timestamp_opt(finished_at as i64, 0)
        .single()
        .map(|dt| dt.format("%Y%m%dT%H%M%S").to_string())
        .unwrap_or_else(|| format!("{finished_at:.0}"));
    let path = dir.join(format!("{ts}.json"));
    let payload = snapshot_to_file(snapshot, session_id, finished_at);
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("spawn_tree.save failed: {e}"))?;
    let index_entry = SpawnTreeIndexEntry {
        path: path.clone(),
        session_id: session_id.to_string(),
        finished_at,
        started_at: payload.started_at,
        label: payload.label.clone(),
        count: snapshot.delegates.len(),
    };
    append_index(&dir, &index_entry);
    Ok(path)
}

pub fn list_archived(session_id: &str, limit: usize) -> Vec<SpawnTreeIndexEntry> {
    let dir = session_dir(session_id);
    let mut entries = read_index(&dir);
    if entries.is_empty() {
        entries = scan_session_dir(&dir);
    } else {
        entries.sort_by(|a, b| {
            b.finished_at
                .partial_cmp(&a.finished_at)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    entries.truncate(limit);
    entries
}

pub fn load_snapshot(path: &str) -> Result<SpawnTurnSnapshot, String> {
    let resolved = resolve_spawn_tree_path(path)?;
    let raw = fs::read_to_string(&resolved).map_err(|e| format!("spawn_tree.load failed: {e}"))?;
    let file: SpawnTreeFile =
        serde_json::from_str(&raw).map_err(|e| format!("spawn_tree.load failed: {e}"))?;
    if file.subagents.is_empty() {
        return Err("snapshot empty or unreadable".into());
    }
    Ok(file_to_snapshot(file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawn_history::{SpawnHistoryEntry, TurnCommitMetrics};
    use std::sync::{Mutex, MutexGuard};
    use tempfile::TempDir;

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        _dir: TempDir,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            TEST_SPAWN_TREES_ROOT.with(|slot| *slot.borrow_mut() = None);
        }
    }

    fn with_temp_home() -> EnvGuard {
        let lock = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("spawn-trees");
        TEST_SPAWN_TREES_ROOT.with(|slot| *slot.borrow_mut() = Some(root));
        EnvGuard {
            _lock: lock,
            _dir: dir,
        }
    }

    fn sample_snapshot() -> SpawnTurnSnapshot {
        SpawnTurnSnapshot::from_entries(
            "goal-a · goal-b".into(),
            vec![
                SpawnHistoryEntry {
                    task_index: 0,
                    task_count: 2,
                    goal: "goal-a".into(),
                    agent_id: "sa-0".into(),
                    parent_id: None,
                    depth: 0,
                    tool_count: 3,
                    duration_secs: 12,
                    status: "completed".into(),
                },
                SpawnHistoryEntry {
                    task_index: 1,
                    task_count: 2,
                    goal: "goal-b".into(),
                    agent_id: "sa-1".into(),
                    parent_id: Some("sa-0".into()),
                    depth: 1,
                    tool_count: 5,
                    duration_secs: 20,
                    status: "completed".into(),
                },
            ],
            TurnCommitMetrics {
                token_est: 900,
                cost_usd: 0.02,
            },
        )
    }

    #[test]
    fn save_load_roundtrip() {
        let _guard = with_temp_home();
        let snapshot = sample_snapshot();
        let path = save_turn_snapshot("sess-1", &snapshot).expect("save");
        assert!(path.is_file());
        let loaded = load_snapshot(path.to_str().unwrap()).expect("load");
        assert_eq!(loaded.delegate_count(), 2);
        assert_eq!(loaded.total_tools, 8);
        assert_eq!(loaded.token_est, 900);
    }

    #[test]
    fn list_returns_saved_entry() {
        let _guard = with_temp_home();
        let snapshot = sample_snapshot();
        let path = save_turn_snapshot("sess-2", &snapshot).expect("save");
        let entries = list_archived("sess-2", 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, path);
        assert_eq!(entries[0].count, 2);
    }

    #[test]
    fn rejects_path_outside_root() {
        let _guard = with_temp_home();
        let _ = fs::create_dir_all(spawn_trees_root());
        assert!(resolve_spawn_tree_path("/etc/passwd").is_err());
    }
}
