//! Hermes-parity edge-case E2E tests for pluggable web search (mock + live where noted).
//!
//! Run:
//! ```bash
//! cargo test -p edgecrab-tools --test web_search_e2e_edge --nocapture
//! cargo test -p edgecrab-tools --test web_search_e2e_edge -- --include-ignored --nocapture
//! ```

mod common;

use common::{
    ctx_with_config, many_mock_results, register_ddgs_mock_success, register_mock, registry_guard,
    test_ctx,
};
use edgecrab_tools::tools::web::search::backend::SearchResult;
use edgecrab_tools::tools::web::search::backends::mock::MockMode;
use edgecrab_tools::{AppConfigRef, ToolHandler, WebSearchTool};
use serde_json::json;

#[tokio::test]
async fn e2e_empty_results_success_not_fallback() {
    let _lock = registry_guard();
    register_mock("empty-hit", MockMode::Success(vec![]));
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "empty-hit".into();
    let result = WebSearchTool
        .execute(json!({"query": "obscure query xyz"}), &ctx_with_config(cfg))
        .await
        .expect("empty results is success");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["backend"], "empty-hit");
    assert!(parsed["results"].as_array().unwrap().is_empty());
    assert!(parsed["fallback_from"].is_null());
}

#[tokio::test]
async fn e2e_400_no_fallback_through_tool() {
    let _lock = registry_guard();
    register_mock("bad-req", MockMode::BadRequest(400));
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "bad-req".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];
    let err = WebSearchTool
        .execute(json!({"query": "test"}), &ctx_with_config(cfg))
        .await
        .expect_err("400 must not fallback");
    assert!(err.to_string().contains("400"));
}

#[tokio::test]
async fn e2e_403_no_fallback_through_tool() {
    let _lock = registry_guard();
    register_mock("forbidden", MockMode::BadRequest(403));
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "forbidden".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];
    let err = WebSearchTool
        .execute(json!({"query": "test"}), &ctx_with_config(cfg))
        .await
        .expect_err("403 must not fallback");
    assert!(err.to_string().contains("403") || err.to_string().contains("simulated"));
}

#[tokio::test]
async fn e2e_503_fallback_through_tool() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("svc-down", MockMode::Server(503));
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "svc-down".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];
    let result = WebSearchTool
        .execute(
            json!({"query": "Paris France", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("503 triggers fallback");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["fallback_from"], "svc-down");
    assert_eq!(parsed["backend"], "ddgs");
}

#[tokio::test]
async fn e2e_timeout_fallback_through_tool() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("timed-out", MockMode::Timeout);
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "timed-out".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];
    let result = WebSearchTool
        .execute(
            json!({"query": "Tokyo Japan", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("timeout triggers fallback");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["fallback_from"], "timed-out");
    assert_eq!(parsed["backend"], "ddgs");
}

#[tokio::test]
async fn e2e_explicit_backend_override_skips_chain() {
    let _lock = registry_guard();
    register_mock("always-429", MockMode::RateLimit);
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "always-429".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];
    let err = WebSearchTool
        .execute(
            json!({"query": "test", "backend": "always-429"}),
            &ctx_with_config(cfg),
        )
        .await
        .expect_err("explicit override does not fall back");
    assert!(err.to_string().contains("429"));
}

#[tokio::test]
async fn e2e_brave_free_alias_resolves() {
    let _lock = registry_guard();
    use edgecrab_tools::config_ref::WebSearchBackendConfigRef;
    use edgecrab_tools::tools::web::search::backend::SearchResult;
    use edgecrab_tools::tools::web::search::backends::mock::MockMode;

    let prev_brave = std::env::var("BRAVE_API_KEY").ok();
    let prev_brave2 = std::env::var("BRAVE_SEARCH_API_KEY").ok();
    unsafe { std::env::remove_var("BRAVE_API_KEY") };
    unsafe { std::env::remove_var("BRAVE_SEARCH_API_KEY") };

    register_mock(
        "brave",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Brave mock",
            "https://example.com/brave",
            "snippet",
            "brave",
        )]),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.backends.insert(
        "brave".into(),
        WebSearchBackendConfigRef {
            api_key: Some("test-key".into()),
            ..Default::default()
        },
    );
    let result = WebSearchTool
        .execute(
            json!({"query": "hello", "backend": "brave-free", "max_results": 1}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("brave-free alias resolves when brave is configured");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "brave");

    register_ddgs_mock_success();
    let result = WebSearchTool
        .execute(
            json!({"query": "hello", "backend": "brave-free", "max_results": 1}),
            &test_ctx(),
        )
        .await
        .expect("brave-free without key degrades to config chain");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["skipped_tool_override"], "brave-free");
    assert_eq!(parsed["backend"], "ddgs");

    if let Some(v) = prev_brave {
        unsafe { std::env::set_var("BRAVE_API_KEY", v) };
    }
    if let Some(v) = prev_brave2 {
        unsafe { std::env::set_var("BRAVE_SEARCH_API_KEY", v) };
    }
}

#[tokio::test]
async fn e2e_env_backend_override() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let prev = std::env::var("EDGECRAB_WEB_SEARCH_BACKEND").ok();
    unsafe { std::env::set_var("EDGECRAB_WEB_SEARCH_BACKEND", "ddgs") };
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();
    let result = WebSearchTool
        .execute(
            json!({"query": "Rust lang", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("env override to ddgs");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
    unsafe { std::env::remove_var("EDGECRAB_WEB_SEARCH_BACKEND") };
    if let Some(v) = prev {
        unsafe { std::env::set_var("EDGECRAB_WEB_SEARCH_BACKEND", v) };
    }
}

#[tokio::test]
async fn e2e_skips_unconfigured_searxng_falls_back_to_ddgs() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    unsafe { std::env::remove_var("SEARXNG_URL") };
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];
    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("skip unconfigured searxng");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
    // searxng is skipped before attempt — not a runtime fallback.
    assert!(parsed["fallback_from"].is_null());
}

#[tokio::test]
async fn e2e_result_shape_parity() {
    let _lock = registry_guard();
    register_mock(
        "shape-mock",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Title",
            "https://example.com/path",
            "snippet text",
            "shape-mock",
        )]),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "shape-mock".into();
    let result = WebSearchTool
        .execute(json!({"query": "shape"}), &ctx_with_config(cfg))
        .await
        .expect("shape mock");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    let row = &parsed["results"][0];
    assert_eq!(row["rank"], 1);
    assert_eq!(row["title"], "Title");
    assert_eq!(row["url"], "https://example.com/path");
    assert_eq!(row["snippet"], "snippet text");
    assert_eq!(row["source"], "shape-mock");
    let hermes = &parsed["data"]["web"][0];
    assert_eq!(hermes["title"], "Title");
    assert_eq!(hermes["url"], "https://example.com/path");
    assert_eq!(hermes["description"], "snippet text");
    assert_eq!(hermes["position"], 1);
    assert_eq!(parsed["success"], true);
}

#[tokio::test]
async fn e2e_plugin_overwrites_same_name() {
    let _lock = registry_guard();
    register_mock(
        "plugin-overwrite",
        MockMode::Success(vec![SearchResult::new(
            1,
            "first",
            "https://first.example",
            "",
            "plugin-overwrite",
        )]),
    );
    register_mock(
        "plugin-overwrite",
        MockMode::Success(vec![SearchResult::new(
            1,
            "second",
            "https://second.example",
            "",
            "plugin-overwrite",
        )]),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "plugin-overwrite".into();
    let result = WebSearchTool
        .execute(json!({"query": "x"}), &ctx_with_config(cfg))
        .await
        .expect("overwrite");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["results"][0]["title"], "second");
}

#[tokio::test]
async fn e2e_unknown_backend_override_errors() {
    let _lock = registry_guard();
    let err = WebSearchTool
        .execute(
            json!({"query": "test", "backend": "nonexistent-backend-xyz"}),
            &test_ctx(),
        )
        .await
        .expect_err("unknown backend");
    assert!(err.to_string().contains("Unknown web search backend"));
}

#[tokio::test]
async fn e2e_invalid_args_missing_query() {
    let _lock = registry_guard();
    let err = WebSearchTool
        .execute(json!({}), &test_ctx())
        .await
        .expect_err("missing query");
    assert!(err.to_string().contains("query") || err.to_string().contains("missing"));
}

#[tokio::test]
async fn e2e_duckduckgo_alias_resolves_to_ddgs() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let result = WebSearchTool
        .execute(
            json!({"query": "Rust programming", "backend": "duckduckgo", "max_results": 2}),
            &test_ctx(),
        )
        .await
        .expect("duckduckgo alias");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
}

#[tokio::test]
async fn e2e_network_error_fallback() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("net-fail", MockMode::Network);
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "net-fail".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];
    let result = WebSearchTool
        .execute(
            json!({"query": "Berlin Germany", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("network fallback");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["fallback_from"], "net-fail");
}

#[tokio::test]
#[ignore = "requires TAVILY_API_KEY"]
async fn e2e_tavily_search_when_key_set() {
    if std::env::var("TAVILY_API_KEY").is_err() {
        eprintln!("Skipping: TAVILY_API_KEY not set");
        return;
    }
    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab agent", "backend": "tavily", "max_results": 3}),
            &test_ctx(),
        )
        .await
        .expect("tavily search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "tavily");
    eprintln!("E2E Tavily proof: {result}");
}

#[tokio::test]
#[ignore = "requires FIRECRAWL_API_KEY"]
async fn e2e_firecrawl_search_when_key_set() {
    if std::env::var("FIRECRAWL_API_KEY").is_err() {
        eprintln!("Skipping: FIRECRAWL_API_KEY not set");
        return;
    }
    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab", "backend": "firecrawl", "max_results": 3}),
            &test_ctx(),
        )
        .await
        .expect("firecrawl search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "firecrawl");
    eprintln!("E2E Firecrawl proof: {result}");
}

#[tokio::test]
async fn e2e_auto_backend_uses_config_chain() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock(
        "chain-primary",
        MockMode::Success(many_mock_results(1, "chain-primary")),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "chain-primary".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(
            json!({"query": "test", "backend": "auto"}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("auto chain");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "chain-primary");
    assert!(parsed["fallback_from"].is_null());
}

#[tokio::test]
async fn e2e_env_web_backend_alias() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let prev = std::env::var("EDGECRAB_WEB_BACKEND").ok();
    unsafe { std::env::set_var("EDGECRAB_WEB_BACKEND", "ddgs") };
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();
    let result = WebSearchTool
        .execute(json!({"query": "alias test"}), &ctx_with_config(cfg))
        .await
        .expect("EDGECRAB_WEB_BACKEND alias");
    unsafe { std::env::remove_var("EDGECRAB_WEB_BACKEND") };
    if let Some(v) = prev {
        unsafe { std::env::set_var("EDGECRAB_WEB_BACKEND", v) };
    }
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
}

#[tokio::test]
async fn e2e_max_results_respected_through_tool() {
    let _lock = registry_guard();
    register_mock(
        "limit-mock",
        MockMode::Success(many_mock_results(10, "limit-mock")),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "limit-mock".into();

    let result = WebSearchTool
        .execute(
            json!({"query": "limit", "max_results": 3}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("limit");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    let results = parsed["results"].as_array().expect("results");
    assert_eq!(results.len(), 3);
    assert_eq!(results[2]["rank"], 3);
}

#[tokio::test]
async fn e2e_max_results_clamped_at_hermes_cap() {
    let _lock = registry_guard();
    register_mock(
        "clamp-mock",
        MockMode::Success(many_mock_results(110, "clamp-mock")),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "clamp-mock".into();
    cfg.result_spill = false;

    let result = WebSearchTool
        .execute(
            json!({"query": "clamp", "max_results": 999}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("clamp");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["results"].as_array().expect("results").len(), 100);
}

#[tokio::test]
async fn e2e_ddg_alias_resolves_to_ddgs() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let result = WebSearchTool
        .execute(
            json!({"query": "Rust", "backend": "ddg", "max_results": 1}),
            &test_ctx(),
        )
        .await
        .expect("ddg alias");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
}

#[tokio::test]
async fn e2e_hard_error_no_fallback() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("hard-fail", MockMode::Hard);
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "hard-fail".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let err = WebSearchTool
        .execute(json!({"query": "test"}), &ctx_with_config(cfg))
        .await
        .expect_err("hard error stops chain");
    assert!(err.to_string().contains("hard failure"));
}

#[tokio::test]
async fn e2e_three_tier_fallback_chain() {
    let _lock = registry_guard();
    register_mock("tier-a", MockMode::Network);
    register_mock("tier-b", MockMode::Server(503));
    register_mock(
        "tier-c",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Third tier win",
            "https://win.example",
            "",
            "tier-c",
        )]),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "tier-a".into();
    cfg.web_search.fallbacks = vec!["tier-b".into(), "tier-c".into()];

    let result = WebSearchTool
        .execute(json!({"query": "chain"}), &ctx_with_config(cfg))
        .await
        .expect("third tier");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "tier-c");
    assert_eq!(parsed["fallback_from"], "tier-a");
}

#[tokio::test]
async fn e2e_no_fallback_from_when_primary_succeeds() {
    let _lock = registry_guard();
    register_mock(
        "winner",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Win",
            "https://win.example",
            "",
            "winner",
        )]),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "winner".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(json!({"query": "ok"}), &ctx_with_config(cfg))
        .await
        .expect("primary win");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "winner");
    assert!(parsed["fallback_from"].is_null());
}

#[tokio::test]
async fn e2e_ddgs_note_present_on_fallback() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("note-src", MockMode::RateLimit);
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "note-src".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(json!({"query": "note test"}), &ctx_with_config(cfg))
        .await
        .expect("ddgs fallback note");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "ddgs");
    let note = parsed["note"].as_str().unwrap_or("");
    assert!(note.contains("fell back"), "note: {note}");
}

#[tokio::test]
async fn e2e_invalid_args_empty_query_string() {
    let _lock = registry_guard();
    register_mock(
        "empty-q",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Empty query ok",
            "https://example.com",
            "",
            "empty-q",
        )]),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "empty-q".into();

    let result = WebSearchTool
        .execute(json!({"query": ""}), &ctx_with_config(cfg))
        .await
        .expect("empty query accepted by tool");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["query"], "");
}

#[tokio::test]
async fn e2e_exa_explicit_unconfigured_degrades_to_chain() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let prev = std::env::var("EXA_API_KEY").ok();
    unsafe { std::env::remove_var("EXA_API_KEY") };
    let result = WebSearchTool
        .execute(json!({"query": "test", "backend": "exa"}), &test_ctx())
        .await
        .expect("exa without key degrades to config chain");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["skipped_tool_override"], "exa");
    assert_eq!(parsed["backend"], "ddgs");
    assert!(
        parsed["note"]
            .as_str()
            .unwrap_or("")
            .contains("Ignored unconfigured backend 'exa'")
    );
    if let Some(v) = prev {
        unsafe { std::env::set_var("EXA_API_KEY", v) };
    }
}

#[tokio::test]
async fn e2e_parallel_explicit_unconfigured_degrades_to_chain() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let prev = std::env::var("PARALLEL_API_KEY").ok();
    unsafe { std::env::remove_var("PARALLEL_API_KEY") };
    let result = WebSearchTool
        .execute(json!({"query": "test", "backend": "parallel"}), &test_ctx())
        .await
        .expect("parallel without key degrades to config chain");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["skipped_tool_override"], "parallel");
    assert_eq!(parsed["backend"], "ddgs");
    assert!(
        parsed["note"]
            .as_str()
            .unwrap_or("")
            .contains("Ignored unconfigured backend 'parallel'")
    );
    if let Some(v) = prev {
        unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
    }
}

#[tokio::test]
#[ignore = "requires EXA_API_KEY"]
async fn e2e_exa_search_when_key_set() {
    if std::env::var("EXA_API_KEY").is_err() {
        eprintln!("Skipping: EXA_API_KEY not set");
        return;
    }
    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab agent", "backend": "exa", "max_results": 3}),
            &test_ctx(),
        )
        .await
        .expect("exa search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "exa");
    assert!(parsed["data"]["web"].as_array().unwrap().len() <= 3);
}

#[tokio::test]
#[ignore = "requires PARALLEL_API_KEY"]
async fn e2e_parallel_search_when_key_set() {
    if std::env::var("PARALLEL_API_KEY").is_err() {
        eprintln!("Skipping: PARALLEL_API_KEY not set");
        return;
    }
    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab agent", "backend": "parallel", "max_results": 3}),
            &test_ctx(),
        )
        .await
        .expect("parallel search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "parallel");
    assert!(parsed["data"]["web"].as_array().unwrap().len() <= 3);
}

#[tokio::test]
async fn e2e_xai_explicit_unconfigured_degrades_to_chain() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    let prev = std::env::var("XAI_API_KEY").ok();
    unsafe { std::env::remove_var("XAI_API_KEY") };
    let result = WebSearchTool
        .execute(json!({"query": "test", "backend": "xai"}), &test_ctx())
        .await
        .expect("xai without key degrades to config chain");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["skipped_tool_override"], "xai");
    assert_eq!(parsed["backend"], "ddgs");
    if let Some(v) = prev {
        unsafe { std::env::set_var("XAI_API_KEY", v) };
    }
}

#[tokio::test]
async fn e2e_env_backend_unconfigured_fail_fast() {
    let _lock = registry_guard();
    let prev_key = std::env::var("PARALLEL_API_KEY").ok();
    let prev_env = std::env::var("EDGECRAB_WEB_SEARCH_BACKEND").ok();
    unsafe { std::env::remove_var("PARALLEL_API_KEY") };
    unsafe { std::env::set_var("EDGECRAB_WEB_SEARCH_BACKEND", "parallel") };
    let err = WebSearchTool
        .execute(json!({"query": "test"}), &test_ctx())
        .await
        .expect_err("env override without key must fail fast");
    assert!(err.to_string().contains("PARALLEL_API_KEY"));
    unsafe { std::env::remove_var("EDGECRAB_WEB_SEARCH_BACKEND") };
    if let Some(v) = prev_env {
        unsafe { std::env::set_var("EDGECRAB_WEB_SEARCH_BACKEND", v) };
    }
    if let Some(v) = prev_key {
        unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
    }
}

#[tokio::test]
#[ignore = "requires XAI_API_KEY"]
async fn e2e_xai_search_when_key_set() {
    if std::env::var("XAI_API_KEY").is_err() {
        eprintln!("Skipping: XAI_API_KEY not set");
        return;
    }
    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab agent", "backend": "xai", "max_results": 3}),
            &test_ctx(),
        )
        .await
        .expect("xai search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "xai");
    assert!(parsed["data"]["web"].as_array().unwrap().len() <= 3);
}
