//! Demo E2E for persistent goals — run from repo root via `demo/persistent-goals/run.sh`.
//!
//! Uses `./demo/persistent-goals/` as the logical workspace and a dedicated
//! `EDGECRAB_HOME` (set by the shell script) so production state is never touched.

use std::sync::Arc;

use edgecrab_core::agent::AgentBuilder;
use edgequake_llm::LLMProvider;

const COPILOT_SPEC: &str = "vscode-copilot/gpt-5-mini";
const DEMO_GOAL: &str = "Refactor demo/persistent-goals/sample_task.md to async/await";

fn demo_db() -> Arc<edgecrab_state::SessionDb> {
    let home = std::env::var("EDGECRAB_HOME").unwrap_or_else(|_| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../demo/.edgecrab-home");
        root.to_string_lossy().into_owned()
    });
    std::fs::create_dir_all(&home).expect("demo home dir");
    let path = std::path::Path::new(&home).join("state.db");
    Arc::new(
        edgecrab_state::SessionDb::open(&path).expect("open demo state.db"),
    )
}

fn copilot_available() -> bool {
    if std::env::var("VSCODE_IPC_HOOK_CLI").is_ok() || std::env::var("VSCODE_COPILOT_TOKEN").is_ok()
    {
        return true;
    }
    dirs::home_dir().is_some_and(|home| {
        [
            home.join(".config/github-copilot/hosts.json"),
            home.join("Library/Application Support/github-copilot/hosts.json"),
        ]
        .iter()
        .any(|p| p.exists())
    })
}

#[tokio::test]
async fn mock_demo_flow() {
    let db = demo_db();
    let provider: Arc<dyn LLMProvider> = Arc::new(edgequake_llm::MockProvider::new());
    let agent = AgentBuilder::new("mock")
        .provider(provider)
        .state_db(db.clone())
        .build()
        .expect("build agent");

    // Reproduces the TUI bug: /goal before any chat turn.
    agent
        .goal_set(DEMO_GOAL)
        .await
        .expect("goal_set must not FK-fail");
    agent
        .subgoal_push("read sample_task.md")
        .await
        .expect("subgoal");
    agent
        .subgoal_push("draft async version")
        .await
        .expect("subgoal 2");

    let show = agent.goal_show().await.expect("show");
    assert!(show.contains("async/await"));
    assert!(show.contains("read sample_task.md"));

    agent.subgoal_done().await.expect("done");
    let after_done = agent.goal_show().await.expect("show after done");
    assert!(after_done.contains("[x] draft async version"));

    agent.force_compress().await;
    let reply = agent
        .chat("Acknowledge the active goal in one short sentence.")
        .await
        .expect("chat after compress");
    assert!(!reply.is_empty());

    let sid = agent
        .session_snapshot()
        .await
        .session_id
        .expect("session id after chat");
    let persisted = db.goals_active(&sid).expect("db goals");
    assert_eq!(persisted.goal_text.as_deref(), Some(DEMO_GOAL));
}

#[tokio::test]
#[ignore = "requires GitHub Copilot auth — run with --ignored"]
async fn copilot_demo_flow() {
    if !copilot_available() {
        eprintln!("SKIP: Copilot credentials not found");
        return;
    }

    let db = demo_db();
    let provider = edgecrab_tools::create_provider_for_model("vscode-copilot", "gpt-5-mini")
        .expect("copilot provider");
    let agent = AgentBuilder::new(COPILOT_SPEC)
        .provider(provider)
        .state_db(db)
        .skip_context_files(true)
        .skip_memory(true)
        .max_iterations(5)
        .build()
        .expect("build copilot agent");

    agent
        .goal_set(DEMO_GOAL)
        .await
        .expect("goal before chat — FK fix");
    agent
        .subgoal_push("summarize sample_task.md")
        .await
        .expect("subgoal");

    let show = agent.goal_show().await.expect("show");
    assert!(show.contains("async/await"), "show:\n{show}");

    let demo_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../demo/persistent-goals/sample_task.md");
    assert!(demo_path.exists(), "demo file missing at {}", demo_path.display());

    let reply = agent
        .chat("What is the active persistent goal for this session? Reply with the goal text only.")
        .await
        .expect("copilot chat");

    eprintln!("Copilot reply: {reply}");
    let lower = reply.to_ascii_lowercase();
    assert!(
        lower.contains("async") || lower.contains("sample_task") || lower.contains("refactor"),
        "model should reflect injected goal context; got: {reply}"
    );
}
