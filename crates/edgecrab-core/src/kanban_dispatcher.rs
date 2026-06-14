//! Kanban dispatcher tick — Hermes `dispatch_once` subset for EdgeCrab.

use edgecrab_state::KanbanDb;
use edgecrab_types::AgentError;

/// Worker spawn request emitted by the dispatcher when a task is claimed.
#[derive(Debug, Clone)]
pub struct KanbanSpawnRequest {
    pub task_id: String,
    pub title: String,
    pub body: Option<String>,
    pub worker_id: String,
}

/// One dispatcher tick outcome.
#[derive(Debug, Clone, Default)]
pub struct KanbanDispatchResult {
    pub reclaimed: usize,
    pub spawned: usize,
    pub skipped_at_capacity: bool,
    pub promoted: usize,
    pub timed_out: usize,
}

/// Dispatcher configuration (from `kanban:` config).
#[derive(Debug, Clone)]
pub struct KanbanDispatchConfig {
    pub claim_ttl_secs: i64,
    pub max_workers: u32,
    pub failure_limit: u32,
}

impl KanbanDispatchConfig {
    pub fn from_kanban_config(cfg: &crate::config::KanbanConfig) -> Self {
        Self {
            claim_ttl_secs: cfg.claim_ttl_secs.clamp(60, 86_400) as i64,
            max_workers: cfg.max_workers.max(1),
            failure_limit: cfg.failure_limit.max(1),
        }
    }
}

/// Run one dispatcher tick: reclaim stale claims, then spawn workers for claimable tasks.
///
/// `spawn` is called after a successful atomic claim. Return `false` to release the claim
/// when spawn fails (e.g. agent unavailable).
pub fn dispatch_once(
    db: &KanbanDb,
    cfg: &KanbanDispatchConfig,
    mut spawn: impl FnMut(KanbanSpawnRequest) -> bool,
) -> Result<KanbanDispatchResult, AgentError> {
    let reclaimed = db.reclaim_stale_claims_with_limit(cfg.failure_limit)?;
    let timed_out = db.enforce_max_runtime_with(cfg.failure_limit, |task_id| {
        crate::kanban_workers::cancel_worker(task_id);
    })?;
    let promoted = db.recompute_ready(cfg.failure_limit)?;
    let mut result = KanbanDispatchResult {
        reclaimed,
        timed_out,
        promoted,
        ..Default::default()
    };

    let doing = db.count_doing_tasks()?;
    if doing >= cfg.max_workers as usize {
        result.skipped_at_capacity = true;
        return Ok(result);
    }

    let slots = cfg.max_workers as usize - doing;
    let candidates = db.list_claimable_tasks(slots)?;

    for task in candidates {
        if db.count_doing_tasks()? >= cfg.max_workers as usize {
            result.skipped_at_capacity = true;
            break;
        }
        let worker_id = format!("dispatcher-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
        if db
            .claim_task(&task.id, &worker_id, cfg.claim_ttl_secs)
            .is_err()
        {
            continue;
        }
        let ok = spawn(KanbanSpawnRequest {
            task_id: task.id.clone(),
            title: task.title.clone(),
            body: task.body.clone(),
            worker_id: worker_id.clone(),
        });
        if !ok {
            let _ = db.release_task(&task.id, Some(&worker_id));
            continue;
        }
        result.spawned += 1;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgecrab_state::KanbanDb;
    use tempfile::TempDir;

    fn test_db() -> (TempDir, std::sync::Arc<KanbanDb>) {
        let dir = TempDir::new().expect("tmpdir");
        let db = KanbanDb::open_default(Some(dir.path())).expect("open");
        (dir, db)
    }

    #[test]
    fn dispatch_respects_parent_dependency() {
        let (_dir, db) = test_db();
        let parent = db.create_task("Parent", None, 0).expect("parent");
        let child = db.create_task("Child", None, 0).expect("child");
        db.link_tasks(&parent.id, &child.id).expect("link");

        let cfg = KanbanDispatchConfig {
            claim_ttl_secs: 900,
            max_workers: 2,
            failure_limit: 2,
        };
        let mut spawned = Vec::new();
        let result = dispatch_once(&db, &cfg, |req| {
            spawned.push(req.task_id);
            true
        })
        .expect("dispatch");
        assert_eq!(result.spawned, 1);
        assert_eq!(spawned, vec![parent.id.clone()]);

        db.complete_task(&parent.id, None, Some("done"))
            .expect("complete parent");
        let result2 = dispatch_once(&db, &cfg, |req| {
            spawned.push(req.task_id);
            true
        })
        .expect("dispatch2");
        assert_eq!(result2.spawned, 1);
        assert!(spawned.contains(&child.id));
    }
}
