#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! Live E2E tests for pluggable web search backends.
//!
//! Run with:
//! ```bash
//! cargo test -p edgecrab-tools --test web_search_e2e -- --include-ignored --nocapture
//! ```

mod common;

use common::{
    DEFAULT_SEARXNG_DOCKER_URL, apply_searxng_docker_env, ctx_with_config, register_ddgs_mock_fail,
    register_ddgs_mock_success, register_mock, registry_guard, searxng_docker_url_if_ready,
    searxng_json_api_ready, test_ctx,
};
use edgecrab_tools::config_ref::WebSearchBackendConfigRef;
use edgecrab_tools::tools::web::search::backend::SearchResult;
use edgecrab_tools::tools::web::search::backends::mock::MockMode;
use edgecrab_tools::{AppConfigRef, ToolHandler, ToolRegistry, WebSearchTool};
use serde_json::json;

#[tokio::test]
#[ignore = "requires internet — live DuckDuckGo HTML search"]
async fn e2e_ddgs_search_without_api_key() {
    let _lock = registry_guard();
    let result = WebSearchTool
        .execute(
            json!({"query": "Rust programming language", "backend": "ddgs", "max_results": 3}),
            &test_ctx(),
        )
        .await;
    match result {
        Ok(body) => {
            let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
            assert_eq!(parsed["success"], true);
            assert_eq!(parsed["backend"], "ddgs");
            let results = parsed["results"].as_array().expect("results array");
            assert!(!results.is_empty(), "ddgs should return at least one hit");
            for hit in results {
                let url = hit["url"].as_str().expect("url field");
                assert!(
                    url.starts_with("http"),
                    "result URL must be absolute and fetchable: {url}"
                );
                assert!(
                    !url.contains("bing.com/ck/a"),
                    "Bing tracking URL must be decoded before web_extract: {url}"
                );
            }
            eprintln!("E2E DDGS proof: {body}");
        }
        Err(e)
            if e.to_string().contains("bot-challenge")
                || e.to_string().contains("bot challenge")
                || e.to_string().contains("blocked this request") =>
        {
            eprintln!(
                "Skipping: DDG returned bot-challenge in this environment (use SEARXNG_URL or BRAVE_API_KEY for reliable live search)"
            );
        }
        Err(e) => panic!("unexpected ddgs error: {e}"),
    }
}

#[tokio::test]
#[ignore = "requires internet — live DuckDuckGo HTML search"]
async fn e2e_ddgs_person_query_never_returns_spam_hits() {
    let _lock = registry_guard();
    let result = WebSearchTool
        .execute(
            json!({"query": "Raphaël MANSUY", "backend": "ddgs", "max_results": 5}),
            &test_ctx(),
        )
        .await;
    match result {
        Ok(body) => {
            let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
            if parsed["success"] != true {
                eprintln!("E2E person query: no results (acceptable): {body}");
                return;
            }
            let results = parsed["results"].as_array().expect("results array");
            for hit in results {
                let title = hit["title"].as_str().unwrap_or("").to_ascii_lowercase();
                let url = hit["url"].as_str().unwrap_or("").to_ascii_lowercase();
                assert!(
                    !title.contains("dexsport") && !url.contains("dexsport"),
                    "must not return Dexsport spam: {hit:?}"
                );
                assert!(
                    !title.contains("tiktok") && !url.contains("tiktok"),
                    "must not return TikTok spam batch: {hit:?}"
                );
                let combined = format!("{title} {}", hit["snippet"].as_str().unwrap_or(""));
                assert!(
                    combined.contains("mansuy"),
                    "relevant person hit must mention MANSUY: {hit:?}"
                );
            }
            eprintln!("E2E person query proof: {body}");
        }
        Err(e)
            if e.to_string().contains("bot-challenge")
                || e.to_string().contains("bot challenge")
                || e.to_string().contains("blocked this request")
                || e.to_string().contains("All metasearch engines failed") =>
        {
            eprintln!("Skipping or blocked (acceptable): {e}");
        }
        Err(e) => panic!("unexpected ddgs error: {e}"),
    }
}

#[tokio::test]
#[ignore = "requires SEARXNG_URL pointing at a reachable public instance"]
async fn e2e_searxng_search_when_configured() {
    let _lock = registry_guard();
    let url = match std::env::var("SEARXNG_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("Skipping: set SEARXNG_URL to a reachable SearXNG instance");
            return;
        }
    };
    unsafe { std::env::set_var("SEARXNG_URL", &url) };

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();
    cfg.web_search.fallbacks = vec![];

    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab agent", "max_results": 3}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("searxng search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["backend"], "searxng");
    eprintln!("E2E SearXNG proof: {result}");
}

#[tokio::test]
#[ignore = "requires Docker SearXNG — run specs/001-gap-analysis-v14/014-web-search-backends/e2e/run-searxng-e2e.sh"]
async fn e2e_searxng_docker_live_search() {
    let _lock = registry_guard();
    let url =
        std::env::var("SEARXNG_URL").unwrap_or_else(|_| DEFAULT_SEARXNG_DOCKER_URL.to_string());

    let docker_e2e = std::env::var("EDGECRAB_SEARXNG_DOCKER_E2E")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"));

    if !docker_e2e && !searxng_json_api_ready(&url) {
        eprintln!(
            "Skipping: start SearXNG with run-searxng-e2e.sh or set EDGECRAB_SEARXNG_DOCKER_E2E=1"
        );
        return;
    }

    if !searxng_json_api_ready(&url) {
        panic!("EDGECRAB_SEARXNG_DOCKER_E2E set but SearXNG not ready at {url}");
    }

    unsafe {
        std::env::set_var("SEARXNG_URL", &url);
        std::env::set_var("EDGECRAB_E2E_SSRF_ALLOW_LOCALHOST", "1");
    }

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();
    cfg.web_search.fallbacks = vec![];

    let result = WebSearchTool
        .execute(
            json!({"query": "Rust programming language", "max_results": 3}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("docker searxng search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["backend"], "searxng");
    let results = parsed["results"].as_array().expect("results");
    assert!(!results.is_empty(), "SearXNG docker should return hits");
    assert!(results[0]["title"].is_string());
    assert!(results[0]["url"].is_string());
    assert_eq!(results[0]["rank"], 1);
    eprintln!("E2E SearXNG Docker proof: {result}");
}

#[tokio::test]
#[ignore = "requires Docker SearXNG — run run-searxng-e2e.sh"]
async fn e2e_searxng_docker_max_results_honored() {
    let _lock = registry_guard();
    let Some(url) = searxng_docker_url_if_ready() else {
        eprintln!("Skipping: SearXNG docker not running");
        return;
    };
    apply_searxng_docker_env(&url);

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();

    let result = WebSearchTool
        .execute(
            json!({"query": "Python programming", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("searxng max_results");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    let results = parsed["results"].as_array().expect("results");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["rank"], 1);
    assert_eq!(results[1]["rank"], 2);
    eprintln!("E2E SearXNG max_results proof: {result}");
}

#[tokio::test]
#[ignore = "requires Docker SearXNG — run run-searxng-e2e.sh"]
async fn e2e_searxng_docker_explicit_backend_override() {
    let _lock = registry_guard();
    let Some(url) = searxng_docker_url_if_ready() else {
        eprintln!("Skipping: SearXNG docker not running");
        return;
    };
    apply_searxng_docker_env(&url);

    let result = WebSearchTool
        .execute(
            json!({"query": "edgecrab agent", "backend": "searxng", "max_results": 3}),
            &test_ctx(),
        )
        .await
        .expect("explicit searxng");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "searxng");
    assert!(parsed["fallback_from"].is_null());
    assert!(!parsed["results"].as_array().unwrap().is_empty());
    eprintln!("E2E SearXNG explicit override proof: {result}");
}

#[tokio::test]
#[ignore = "requires Docker SearXNG — run run-searxng-e2e.sh"]
async fn e2e_searxng_docker_config_endpoint_without_env() {
    let _lock = registry_guard();
    let Some(url) = searxng_docker_url_if_ready() else {
        eprintln!("Skipping: SearXNG docker not running");
        return;
    };
    unsafe {
        std::env::remove_var("SEARXNG_URL");
        std::env::set_var("EDGECRAB_E2E_SSRF_ALLOW_LOCALHOST", "1");
    }

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();
    cfg.web_search.backends.insert(
        "searxng".into(),
        WebSearchBackendConfigRef {
            endpoint: Some(url.clone()),
            ..Default::default()
        },
    );

    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab web search", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("config endpoint searxng");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "searxng");
    assert!(!parsed["results"].as_array().unwrap().is_empty());
    let web = parsed["data"]["web"].as_array().expect("hermes data.web");
    assert!(!web.is_empty());
    assert!(web[0]["description"].is_string());
    eprintln!("E2E SearXNG config endpoint proof: {result}");
}

#[tokio::test]
#[ignore = "requires Docker SearXNG — run run-searxng-e2e.sh"]
async fn e2e_searxng_docker_fallback_from_failing_mock() {
    let _lock = registry_guard();
    let Some(url) = searxng_docker_url_if_ready() else {
        eprintln!("Skipping: SearXNG docker not running");
        return;
    };
    apply_searxng_docker_env(&url);
    register_mock("primary-down", MockMode::Server(503));

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "primary-down".into();
    cfg.web_search.fallbacks = vec!["searxng".into()];

    let result = WebSearchTool
        .execute(
            json!({"query": "Rust language", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("503 mock -> searxng docker");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["fallback_from"], "primary-down");
    assert_eq!(parsed["backend"], "searxng");
    assert!(!parsed["results"].as_array().unwrap().is_empty());
    eprintln!("E2E SearXNG docker fallback proof: {result}");
}

#[tokio::test]
#[ignore = "requires Docker SearXNG — run run-searxng-e2e.sh"]
async fn e2e_searxng_docker_auto_backend_uses_config_chain() {
    let _lock = registry_guard();
    let Some(url) = searxng_docker_url_if_ready() else {
        eprintln!("Skipping: SearXNG docker not running");
        return;
    };
    apply_searxng_docker_env(&url);
    register_ddgs_mock_success();

    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "searxng".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(
            json!({"query": "Tokyo Japan", "backend": "auto", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("auto uses searxng first");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "searxng");
    assert!(parsed["fallback_from"].is_null());
    eprintln!("E2E SearXNG auto chain proof: {result}");
}

#[tokio::test]
#[ignore = "requires BRAVE_API_KEY"]
async fn e2e_brave_search_when_key_set() {
    let _lock = registry_guard();
    if std::env::var("BRAVE_API_KEY")
        .or_else(|_| std::env::var("BRAVE_SEARCH_API_KEY"))
        .is_err()
    {
        eprintln!("Skipping: BRAVE_API_KEY not set");
        return;
    }
    let result = WebSearchTool
        .execute(
            json!({"query": "EdgeCrab", "backend": "brave", "max_results": 3}),
            &test_ctx(),
        )
        .await
        .expect("brave search");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "brave");
    eprintln!("E2E Brave proof: {result}");
}

#[tokio::test]
async fn e2e_fallback_from_rate_limited_primary() {
    let _lock = registry_guard();
    register_ddgs_mock_success();
    register_mock("always-429", MockMode::RateLimit);
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "always-429".into();
    cfg.web_search.fallbacks = vec!["ddgs".into()];

    let result = WebSearchTool
        .execute(
            json!({"query": "Paris France", "max_results": 2}),
            &ctx_with_config(cfg),
        )
        .await
        .expect("fallback to ddgs");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["fallback_from"], "always-429");
    assert_eq!(parsed["backend"], "ddgs");
    eprintln!("E2E fallback proof: {result}");
}

#[tokio::test]
async fn e2e_all_fail_returns_descriptive_error() {
    let _lock = registry_guard();
    register_mock("fail-one", MockMode::Network);
    register_mock("fail-two", MockMode::Server(503));
    register_ddgs_mock_fail();
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "fail-one".into();
    cfg.web_search.fallbacks = vec!["fail-two".into()];

    let err = WebSearchTool
        .execute(json!({"query": "test"}), &ctx_with_config(cfg))
        .await
        .expect_err("all backends fail");
    let msg = err.to_string();
    assert!(msg.contains("fail-one"));
    assert!(msg.contains("fail-two"));
    eprintln!("E2E all-fail proof: {msg}");
}

#[tokio::test]
async fn e2e_plugin_registered_backend_via_tool() {
    let _lock = registry_guard();
    register_mock(
        "plugin-demo",
        MockMode::Success(vec![SearchResult::new(
            1,
            "Plugin Hit",
            "https://plugin.example",
            "from plugin",
            "plugin-demo",
        )]),
    );
    let mut cfg = AppConfigRef::default();
    cfg.web_search.primary = "plugin-demo".into();

    let result = WebSearchTool
        .execute(json!({"query": "anything"}), &ctx_with_config(cfg))
        .await
        .expect("plugin backend");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["backend"], "plugin-demo");
    assert_eq!(parsed["results"][0]["title"], "Plugin Hit");
}

#[test]
fn e2e_tool_registered_in_registry() {
    let registry = ToolRegistry::new();
    assert!(registry.tool_names().contains(&"web_search"));
}
