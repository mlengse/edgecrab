//! E2E: Copilot gpt-5-mini agent invokes `web_search` with pluggable backends.

use edgecrab_core::agent::AgentBuilder;
use edgequake_llm::providers::vscode::{auth::GitHubAuth, token::TokenManager};

const COPILOT_SPEC: &str = "vscode-copilot/gpt-5-mini";

fn copilot_available() -> bool {
    std::env::var("VSCODE_IPC_HOOK_CLI").is_ok()
        || std::env::var("VSCODE_COPILOT_TOKEN").is_ok()
        || dirs::home_dir()
            .map(|h| h.join(".config/github-copilot/hosts.json").exists())
            .unwrap_or(false)
}

async fn ensure_copilot_auth() {
    let manager = TokenManager::new().expect("token manager");
    if manager.get_valid_copilot_token().await.is_ok() {
        return;
    }
    let auth = GitHubAuth::new().expect("github auth");
    let _ = auth
        .device_code_flow(|code| {
            eprintln!("Copilot auth: {}", code.verification_uri);
            eprintln!("Code: {}", code.user_code);
        })
        .await;
}

#[tokio::test]
#[ignore = "requires VS Code Copilot + network"]
async fn e2e_agent_web_search_with_copilot_gpt5_mini() {
    if !copilot_available() {
        eprintln!("Skipping: Copilot not available");
        return;
    }
    ensure_copilot_auth().await;

    let provider = edgecrab_tools::create_provider_for_model("vscode-copilot", "gpt-5-mini")
        .expect("provider");

    let registry = std::sync::Arc::new(edgecrab_tools::ToolRegistry::new());

    let agent = AgentBuilder::new(COPILOT_SPEC)
        .provider(provider)
        .tools(registry)
        .max_iterations(8)
        .build()
        .expect("agent");

    let reply = agent
        .chat(
            "Use the web_search tool exactly once with query 'Rust async book' and backend 'ddgs'. \
             After the tool returns, reply with the backend field value and the title of the first result only.",
        )
        .await
        .expect("agent chat");

    assert!(
        reply.to_lowercase().contains("ddgs") || reply.to_lowercase().contains("rust"),
        "agent should mention ddgs backend or rust result; got: {reply}"
    );
    eprintln!("Copilot gpt-5-mini web_search E2E proof:\n{reply}");
}

#[tokio::test]
#[ignore = "requires VS Code Copilot + network"]
async fn e2e_agent_web_search_fallback_chain_ddgs() {
    if !copilot_available() {
        eprintln!("Skipping: Copilot not available");
        return;
    }
    ensure_copilot_auth().await;

    let provider = edgecrab_tools::create_provider_for_model("vscode-copilot", "gpt-5-mini")
        .expect("provider");
    let registry = std::sync::Arc::new(edgecrab_tools::ToolRegistry::new());

    let agent = AgentBuilder::new(COPILOT_SPEC)
        .provider(provider)
        .tools(registry)
        .max_iterations(10)
        .build()
        .expect("agent");

    let reply = agent
        .chat(
            "Use the web_search tool once with query 'Tokyo Japan weather' and backend 'ddgs'. \
             Do not use any other tools. Reply with only the backend name and first result title, separated by ' — '.",
        )
        .await
        .expect("agent chat");

    let lower = reply.to_lowercase();
    assert!(
        lower.contains("ddgs") || lower.contains("tokyo") || lower.contains("weather"),
        "agent should report ddgs or a relevant hit; got: {reply}"
    );
    eprintln!("Copilot fallback-chain E2E proof:\n{reply}");
}

#[tokio::test]
#[ignore = "requires VS Code Copilot + Docker SearXNG (run run-searxng-e2e.sh first)"]
async fn e2e_agent_web_search_searxng_docker() {
    if !copilot_available() {
        eprintln!("Skipping: Copilot not available");
        return;
    }

    let searxng_url = std::env::var("SEARXNG_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8888".to_string());

    if !curl_searxng_ready(&searxng_url) {
        eprintln!("Skipping: SearXNG not ready at {searxng_url}");
        return;
    }

    unsafe {
        std::env::set_var("SEARXNG_URL", &searxng_url);
        std::env::set_var("EDGECRAB_E2E_SSRF_ALLOW_LOCALHOST", "1");
    }

    ensure_copilot_auth().await;

    let provider = edgecrab_tools::create_provider_for_model("vscode-copilot", "gpt-5-mini")
        .expect("provider");
    let registry = std::sync::Arc::new(edgecrab_tools::ToolRegistry::new());

    let agent = AgentBuilder::new(COPILOT_SPEC)
        .provider(provider)
        .tools(registry)
        .max_iterations(10)
        .build()
        .expect("agent");

    let reply = agent
        .chat(
            "Use the web_search tool exactly once with query 'EdgeCrab agent' and backend 'searxng'. \
             Reply with only: backend name, then ' — ', then the first result title.",
        )
        .await
        .expect("agent chat");

    let lower = reply.to_lowercase();
    assert!(
        lower.contains("searxng") || lower.contains("edgecrab"),
        "agent should report searxng or a relevant hit; got: {reply}"
    );
    eprintln!("Copilot SearXNG docker E2E proof:\n{reply}");
}

fn curl_searxng_ready(base_url: &str) -> bool {
    let url = format!(
        "{}/search?q=edgecrab&format=json",
        base_url.trim_end_matches('/')
    );
    std::process::Command::new("curl")
        .args(["-sf", "--max-time", "5", &url])
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
}
