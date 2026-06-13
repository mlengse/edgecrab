#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: runtime web_search — disk SSoT, fallback notes, stale session edge cases.

mod common;

use common::{
    ctx_with_config, edgecrab_home_guard, register_ddgs_mock_success, register_mock, registry_guard,
};
use edgecrab_tools::{
    AppConfigRef, ToolHandler, WebSearchChainUpdate, WebSearchTool, effective_web_search_config,
    format_saved_chain_summary, format_search_chain_summary, persist_web_search_chain_in_config,
    tools::web::search::backend::SearchResult, tools::web::search::backends::mock::MockMode,
};
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn e2e_disk_chain_used_when_session_snapshot_stale() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let _home = edgecrab_home_guard(dir.path());
    let path = dir.path().join("config.yaml");
    persist_web_search_chain_in_config(
        &path,
        &WebSearchChainUpdate {
            primary: Some("disk-primary".into()),
            fallbacks: Some(vec!["ddgs".into()]),
            timeout_secs: None,
        },
    )
    .expect("persist");

    register_mock(
        "disk-primary",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Disk primary hit",
            "https://example.com/disk",
            "",
            "disk-primary",
        )]),
    );
    register_ddgs_mock_success();

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "session-only".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(json!({"query": "runtime chain"}), &ctx_with_config(cfg))
        .await
        .expect("disk chain wins");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "disk-primary");
    assert!(parsed["fallback_from"].is_null());
}

#[tokio::test]
async fn e2e_fallback_note_mentions_failed_primary() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("fail-primary", MockMode::RateLimit);
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "fail-primary".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(json!({"query": "fallback note"}), &ctx_with_config(cfg))
        .await
        .expect("ddgs fallback");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
    assert_eq!(parsed["fallback_from"], "fail-primary");
    let note = parsed["note"].as_str().unwrap_or("");
    assert!(note.contains("fail-primary"), "note: {note}");
    assert!(note.contains("fell back"));
}

#[tokio::test]
async fn e2e_ddgs_note_suppressed_when_tavily_key_set_and_no_fallback() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let prev = std::env::var("TAVILY_API_KEY").ok();
    unsafe { std::env::set_var("TAVILY_API_KEY", "test-tavily-key") };

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "ddgs".into();

    let result = WebSearchTool
        .execute(json!({"query": "ddgs only"}), &ctx_with_config(cfg))
        .await
        .expect("ddgs search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
    assert!(parsed["note"].is_null());

    unsafe { std::env::remove_var("TAVILY_API_KEY") };
    if let Some(v) = prev {
        unsafe { std::env::set_var("TAVILY_API_KEY", v) };
    }
}

#[tokio::test]
async fn e2e_skipped_override_plus_primary_fallback() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("fail-tavily", MockMode::RateLimit);
    let prev = std::env::var("PARALLEL_API_KEY").ok();
    unsafe { std::env::remove_var("PARALLEL_API_KEY") };
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "fail-tavily".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(
            json!({"query": "person name", "backend": "parallel"}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("parallel skipped, tavily fails, ddgs wins");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["skipped_tool_override"], "parallel");
    assert_eq!(parsed["backend"], "ddgs");
    assert_eq!(parsed["fallback_from"], "fail-tavily");
    let note = parsed["note"].as_str().unwrap_or("");
    assert!(note.contains("Ignored unconfigured backend 'parallel'"));
    assert!(note.contains("fail-tavily"));
    if let Some(v) = prev {
        unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
    }
}

#[tokio::test]
async fn e2e_homelab_chain_parallel_tool_arg_degrades() {
    let _lock = registry_guard();
    register_mock(
        "tavily",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Tavily hit",
            "https://example.com/t",
            "snippet",
            "tavily",
        )]),
    );
    register_ddgs_mock_success();
    let prev = std::env::var("PARALLEL_API_KEY").ok();
    unsafe { std::env::remove_var("PARALLEL_API_KEY") };
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "tavily".into();
    cfg.web_search.fallbacks = vec!["brave".into(), "ddgs".into(), "firecrawl".into()];
    cfg.web_search.backends.insert(
        "tavily".into(),
        edgecrab_tools::config_ref::WebSearchBackendConfigRef {
            api_key: Some("mock-tavily".into()),
            ..Default::default()
        },
    );

    let result = WebSearchTool
        .execute(
            json!({"query": "Raphaël MANSUY", "backend": "parallel"}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("homelab-like: ignore parallel, use tavily");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["skipped_tool_override"], "parallel");
    assert_eq!(parsed["backend"], "tavily");
    assert!(parsed["fallback_from"].is_null());
    if let Some(v) = prev {
        unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
    }
}

#[test]
fn e2e_saved_vs_effective_chain_summary() {
    let cfg = edgecrab_tools::config_ref::WebSearchConfigRef {
        primary: "searxng".into(),
        fallbacks: vec!["brave".into(), "ddgs".into()],
        timeout_secs: 12,
        ..Default::default()
    };
    assert_eq!(
        format_saved_chain_summary(&cfg),
        "searxng → brave → ddgs (12s timeout)"
    );
    assert_eq!(format_search_chain_summary(&cfg), "ddgs (12s timeout)");
}

#[test]
fn e2e_effective_config_matches_disk_after_persist() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let _home = edgecrab_home_guard(dir.path());
    let path = dir.path().join("config.yaml");
    persist_web_search_chain_in_config(
        &path,
        &WebSearchChainUpdate {
            primary: Some("tavily".into()),
            fallbacks: Some(vec!["brave".into(), "ddgs".into()]),
            timeout_secs: Some(12),
        },
    )
    .expect("persist");

    let session = AppConfigRef::default().web_search;
    let effective = effective_web_search_config(&session);
    assert_eq!(effective.primary, "tavily");
    assert_eq!(effective.fallbacks, vec!["brave", "ddgs"]);
    assert_eq!(effective.timeout_secs, 12);
}
