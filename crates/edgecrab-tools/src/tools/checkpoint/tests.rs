//! Checkpoint v2 integration tests.

use std::path::Path;

use serde_json::json;

use crate::registry::{ToolContext, ToolHandler};
use crate::test_support::TestEdgecrabHome;
use crate::tools::checkpoint::{
    CheckpointConfig, CheckpointTool, checkpoint_new_turn, excludes, git,
    manager::CheckpointManager, maybe_auto_prune_checkpoints,
};

fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .status()
        .is_ok()
}

fn init_git_repo(dir: &Path) {
    let _ = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status();
    let _ = std::process::Command::new("git")
        .args(["config", "user.email", "test@test"])
        .current_dir(dir)
        .status();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", "test"])
        .current_dir(dir)
        .status();
}

fn commit_all(dir: &Path, msg: &str) {
    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .status();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", msg, "--quiet"])
        .current_dir(dir)
        .status();
}

fn test_ctx(work: &tempfile::TempDir, home: &tempfile::TempDir) -> ToolContext {
    let mut ctx = ToolContext::test_context();
    ctx.cwd = work.path().to_path_buf();
    ctx.config.edgecrab_home = home.path().to_path_buf();
    ctx.config.checkpoints_enabled = true;
    ctx.config.checkpoints_max_snapshots = 20;
    ctx.config.checkpoints_max_total_size_mb = 200;
    ctx.config.checkpoints_max_file_size_mb = 10;
    ctx
}

#[test]
fn tool_metadata() {
    let tool = CheckpointTool;
    assert_eq!(tool.name(), "checkpoint");
    assert_eq!(tool.toolset(), "core");
    assert!(tool.is_available());
}

#[tokio::test]
async fn create_and_list_checkpoint() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let home = tempfile::TempDir::new().expect("home");
    init_git_repo(work.path());
    std::fs::write(work.path().join("test_ckpt.txt"), "hello checkpoint").expect("write");
    commit_all(work.path(), "init");

    let ctx = test_ctx(&work, &home);
    checkpoint_new_turn();
    let result = CheckpointTool
        .execute(json!({ "action": "create", "name": "test-ckpt" }), &ctx)
        .await
        .expect("create");
    assert!(result.contains("created"), "got: {result}");

    let list = CheckpointTool
        .execute(json!({ "action": "list" }), &ctx)
        .await
        .expect("list");
    assert!(list.contains("test-ckpt"), "got: {list}");
    assert!(list.contains("size_bytes"), "got: {list}");
}

#[tokio::test]
async fn diff_no_changes() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let home = tempfile::TempDir::new().expect("home");
    init_git_repo(work.path());
    std::fs::write(work.path().join("test_diff.txt"), "diff content").expect("write");
    commit_all(work.path(), "init");

    let ctx = test_ctx(&work, &home);
    checkpoint_new_turn();
    CheckpointTool
        .execute(json!({ "action": "create", "name": "diff-test" }), &ctx)
        .await
        .expect("create");

    let diff = CheckpointTool
        .execute(json!({ "action": "diff", "n": 1 }), &ctx)
        .await
        .expect("diff");
    assert!(diff.contains("no changes"), "got: {diff}");
}

#[tokio::test]
async fn invalid_action() {
    let ctx = ToolContext::test_context();
    let result = CheckpointTool
        .execute(json!({ "action": "explode" }), &ctx)
        .await;
    assert!(matches!(
        result,
        Err(edgecrab_types::ToolError::InvalidArgs { .. })
    ));
}

#[tokio::test]
async fn checkpoint_uses_context_home_not_process_env() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let configured_home = tempfile::TempDir::new().expect("configured");
    let foreign_home = TestEdgecrabHome::new();
    init_git_repo(work.path());
    std::fs::write(work.path().join("test_ckpt.txt"), "hello").expect("write");
    commit_all(work.path(), "init");

    let ctx = test_ctx(&work, &configured_home);
    checkpoint_new_turn();
    CheckpointTool
        .execute(json!({ "action": "create", "name": "ctx-home" }), &ctx)
        .await
        .expect("create");

    let store = git::store_path(&git::checkpoint_base(configured_home.path()));
    assert!(store.join("HEAD").exists(), "should use configured home");
    let foreign_store = git::store_path(&git::checkpoint_base(foreign_home.path()));
    assert!(!foreign_store.join("HEAD").exists());
}

#[test]
fn default_excludes_module() {
    let content = excludes::default_exclude_file_content();
    assert!(content.contains("node_modules/"));
    assert!(content.contains("target/"));
}

#[test]
fn project_hash_deterministic() {
    let tmp = tempfile::TempDir::new().expect("tmp");
    let a = git::project_hash(tmp.path());
    let b = git::project_hash(tmp.path());
    assert_eq!(a, b);
    assert_eq!(a.len(), 16);
}

#[test]
fn validate_commit_hash_rejects_injection() {
    assert!(git::validate_commit_hash("--help").is_some());
    assert!(git::validate_commit_hash("").is_some());
    assert!(git::validate_commit_hash("abc1234").is_none());
}

#[test]
fn validate_file_path_rejects_traversal() {
    let tmp = tempfile::TempDir::new().expect("tmp");
    assert!(git::validate_file_path("../etc/passwd", tmp.path()).is_some());
    assert!(git::validate_file_path("src/main.rs", tmp.path()).is_none());
}

#[test]
fn eviction_keeps_max_snapshots() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let home = tempfile::TempDir::new().expect("home");
    init_git_repo(work.path());
    std::fs::write(work.path().join("f.txt"), "v0").expect("write");
    commit_all(work.path(), "init");

    let cfg = CheckpointConfig {
        enabled: true,
        max_snapshots: 5,
        max_total_size_mb: 0,
        max_file_size_mb: 0,
        edgecrab_home: home.path().to_path_buf(),
    };
    let mut mgr = CheckpointManager::new(cfg);

    for i in 0..8 {
        std::fs::write(work.path().join("f.txt"), format!("v{i}")).expect("write");
        let _ = mgr.save(work.path(), &format!("snap {i}"));
    }
    let entries = mgr.list_checkpoints(work.path());
    assert!(
        entries.len() <= 5,
        "expected <=5 checkpoints after eviction, got {}",
        entries.len()
    );
}

#[test]
fn pin_survives_eviction() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let home = tempfile::TempDir::new().expect("home");
    init_git_repo(work.path());
    std::fs::write(work.path().join("f.txt"), "v0").expect("write");
    commit_all(work.path(), "init");

    let cfg = CheckpointConfig {
        enabled: true,
        max_snapshots: 3,
        max_total_size_mb: 0,
        max_file_size_mb: 0,
        edgecrab_home: home.path().to_path_buf(),
    };
    let mgr = CheckpointManager::new(cfg.clone());

    for i in 0..5 {
        std::fs::write(work.path().join("f.txt"), format!("v{i}")).expect("write");
        let mut m = CheckpointManager::new(cfg.clone());
        let _ = m.save(work.path(), &format!("snap {i}"));
    }

    let entries = mgr.list_checkpoints(work.path());
    let oldest = entries.last().expect("has entries");
    mgr.pin_checkpoint(work.path(), oldest.n, true)
        .expect("pin");

    for i in 5..10 {
        std::fs::write(work.path().join("f.txt"), format!("v{i}")).expect("write");
        let mut m = CheckpointManager::new(cfg.clone());
        let _ = m.save(work.path(), &format!("snap {i}"));
    }

    let after = mgr.list_checkpoints(work.path());
    assert!(
        after.iter().any(|e| e.hash == oldest.hash),
        "pinned checkpoint should survive eviction"
    );
}

#[test]
fn excludes_node_modules_from_snapshot() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let home = tempfile::TempDir::new().expect("home");
    init_git_repo(work.path());
    std::fs::write(work.path().join("main.rs"), "fn main() {}").expect("write");
    let nm = work.path().join("node_modules/pkg/index.js");
    std::fs::create_dir_all(nm.parent().unwrap()).expect("mkdir");
    std::fs::write(&nm, "ignored").expect("write");
    commit_all(work.path(), "init");

    let cfg = CheckpointConfig {
        enabled: true,
        max_snapshots: 20,
        max_total_size_mb: 0,
        max_file_size_mb: 0,
        edgecrab_home: home.path().to_path_buf(),
    };
    let mut mgr = CheckpointManager::new(cfg);
    let _ = mgr.save(work.path(), "with excludes");

    let base = git::checkpoint_base(home.path());
    let store = git::store_path(&base);
    let abs = git::normalize_path(work.path());
    let index = git::index_path(&store, &git::project_hash(&abs));
    let ls = git::run_git(
        &["ls-files", "--cached"],
        &store,
        &abs,
        Some(&index),
        &std::collections::HashSet::new(),
        git::GIT_TIMEOUT_SECS,
    );
    assert!(ls.ok);
    assert!(!ls.stdout.contains("node_modules"), "got: {}", ls.stdout);
}

#[test]
fn auto_prune_idempotent_marker() {
    let home = tempfile::TempDir::new().expect("home");
    std::fs::create_dir_all(home.path().join("checkpoints")).expect("mkdir");
    let first = maybe_auto_prune_checkpoints(home.path(), 7, 24, true, 200);
    assert!(!first.skipped);
    let second = maybe_auto_prune_checkpoints(home.path(), 7, 24, true, 200);
    assert!(second.skipped);
}

#[test]
fn restore_emits_mutation_records() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let home = tempfile::TempDir::new().expect("home");
    init_git_repo(work.path());
    std::fs::write(work.path().join("a.txt"), "original").expect("write");
    commit_all(work.path(), "init");

    let cfg = CheckpointConfig {
        enabled: true,
        max_snapshots: 20,
        max_total_size_mb: 0,
        max_file_size_mb: 0,
        edgecrab_home: home.path().to_path_buf(),
    };
    let mut mgr = CheckpointManager::new(cfg);
    let _ = mgr.save(work.path(), "before edit");
    std::fs::write(work.path().join("a.txt"), "edited").expect("write");

    let entries = mgr.list_checkpoints(work.path());
    let hash = entries[0].hash.clone();

    let mut ctx = test_ctx(&work, &home);
    let mutation = std::sync::Arc::new(crate::mutations::MutationTurnState::new());
    ctx.mutation_turn = Some(mutation.clone());

    mgr.restore(work.path(), &hash, None, Some(&ctx))
        .expect("restore");
    let records = mutation.drain_success();
    assert!(!records.is_empty(), "restore should record mutations");
}

#[test]
fn rollback_handler_list_when_empty() {
    let home = tempfile::TempDir::new().expect("home");
    let work = tempfile::TempDir::new().expect("work");
    let cfg = CheckpointConfig::from_home(home.path().to_path_buf(), true, 20, 200, 10);
    let outcome = super::rollback::handle_rollback_command("", work.path(), cfg);
    assert!(matches!(
        outcome,
        super::rollback::RollbackOutcome::Report { .. }
    ));
}

#[test]
fn rollback_handler_restore_by_number() {
    if !git_available() {
        return;
    }
    let work = tempfile::TempDir::new().expect("work");
    let home = tempfile::TempDir::new().expect("home");
    init_git_repo(work.path());
    std::fs::write(work.path().join("state.txt"), "v1").expect("write");
    commit_all(work.path(), "init");

    let cfg = CheckpointConfig::from_home(home.path().to_path_buf(), true, 20, 200, 10);
    let mut mgr = CheckpointManager::new(cfg.clone());
    let _ = mgr.save(work.path(), "snapshot v1");
    std::fs::write(work.path().join("state.txt"), "v2").expect("write");

    let list = super::rollback::handle_rollback_command("list", work.path(), cfg.clone());
    let body = match list {
        super::rollback::RollbackOutcome::Report { body, .. } => body,
        other => panic!("expected list report, got {other:?}"),
    };
    assert!(body.contains("snapshot v1"), "list body: {body}");

    let restore = super::rollback::handle_rollback_command("1", work.path(), cfg.clone());
    match restore {
        super::rollback::RollbackOutcome::System(msg) => {
            assert!(msg.contains("Restored"), "restore msg: {msg}");
        }
        other => panic!("expected restore system msg, got {other:?}"),
    }
    let content = std::fs::read_to_string(work.path().join("state.txt")).expect("read");
    assert_eq!(content, "v1");

    let status = super::maintenance::store_status(home.path());
    assert_eq!(status.project_count, 1);
    assert!(status.store_size_bytes > 0);
}

#[test]
fn store_status_empty_base() {
    let home = tempfile::TempDir::new().expect("home");
    let status = super::maintenance::store_status(home.path());
    assert_eq!(status.project_count, 0);
    assert_eq!(status.total_size_bytes, 0);
}

#[test]
fn clear_all_removes_base() {
    let home = tempfile::TempDir::new().expect("home");
    std::fs::create_dir_all(home.path().join("checkpoints/store")).expect("mkdir");
    let result = super::maintenance::clear_all(home.path());
    assert!(result.deleted);
    assert!(!home.path().join("checkpoints").exists());
}

#[test]
fn two_session_isolation() {
    if !git_available() {
        return;
    }
    let work_a = tempfile::TempDir::new().expect("work_a");
    let work_b = tempfile::TempDir::new().expect("work_b");
    let home = tempfile::TempDir::new().expect("home");
    for w in [&work_a, &work_b] {
        init_git_repo(w.path());
        std::fs::write(w.path().join("f.txt"), "x").expect("write");
        commit_all(w.path(), "init");
    }

    let cfg = CheckpointConfig {
        enabled: true,
        max_snapshots: 20,
        max_total_size_mb: 0,
        max_file_size_mb: 0,
        edgecrab_home: home.path().to_path_buf(),
    };
    let mut ma = CheckpointManager::new(cfg.clone());
    let mut mb = CheckpointManager::new(cfg);
    let _ = ma.save(work_a.path(), "a-only");
    let _ = mb.save(work_b.path(), "b-only");

    let la = ma.list_checkpoints(work_a.path());
    let lb = mb.list_checkpoints(work_b.path());
    assert_eq!(la.len(), 1);
    assert_eq!(lb.len(), 1);
    assert_ne!(la[0].hash, lb[0].hash);
}
