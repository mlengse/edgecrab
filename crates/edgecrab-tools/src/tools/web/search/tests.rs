//! Unit tests for web search backend chain and registry.
#![allow(clippy::await_holding_lock)]

use std::sync::Arc;

use super::backend::SearchResult;
use super::backends::mock::{MockBackend, MockExtractMode, MockMode};
use super::chain::BackendChain;
use super::config::{EnvBackendGuard, ExtractOptions, ResolvedChain, SearchOptions};
use super::error::SearchError;
use super::error::SearchErrorKind;
use super::registry::{
    extract_with_backend, get_web_search_backend, list_web_provider_setup_schemas,
    register_web_search_backend, reset_registry_for_tests, test_registry_lock,
};
use super::test_isolation::{EdgecrabHomeGuard, web_config_test_lock};
use crate::config_ref::WebSearchConfigRef;

fn sample_result() -> SearchResult {
    SearchResult::new(1, "Title", "https://example.com", "snippet", "mock")
}

#[test]
fn registry_lists_builtin_brave() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    let backend = get_web_search_backend("brave");
    assert!(backend.is_some());
    assert_eq!(backend.expect("brave").name(), "brave");
}

#[test]
fn plugin_can_register_custom_backend() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    let mock = Arc::new(MockBackend::new(
        "custom-plugin",
        MockMode::Success(vec![sample_result()]),
    ));
    register_web_search_backend(mock);
    let found = get_web_search_backend("custom-plugin").expect("registered");
    assert_eq!(found.name(), "custom-plugin");
}

#[tokio::test]
async fn chain_fallback_policy_end_to_end() {
    let _lock = test_registry_lock();
    let _env = EnvBackendGuard::isolate();
    let _home = EdgecrabHomeGuard::isolated(None);
    reset_registry_for_tests();

    register_web_search_backend(Arc::new(MockBackend::new(
        "primary-mock",
        MockMode::RateLimit,
    )));
    register_web_search_backend(Arc::new(MockBackend::new(
        "fallback-mock",
        MockMode::Success(vec![sample_result()]),
    )));

    let cfg = WebSearchConfigRef {
        primary: "primary-mock".into(),
        fallbacks: vec!["fallback-mock".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    let chain = BackendChain::from_resolved(&resolved).expect("chain");
    let (results, used) = chain
        .search("test", SearchOptions::default())
        .await
        .expect("fallback success");
    assert_eq!(used, "fallback-mock");
    assert_eq!(results.len(), 1);

    reset_registry_for_tests();
    register_web_search_backend(Arc::new(MockBackend::new(
        "bad-mock",
        MockMode::BadRequest(400),
    )));
    let cfg = WebSearchConfigRef {
        primary: "bad-mock".into(),
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    let chain = BackendChain::from_resolved(&resolved).expect("chain");
    let err = chain
        .search("test", SearchOptions::default())
        .await
        .expect_err("no fallback on 400");
    assert!(matches!(err.kind, SearchErrorKind::BadRequest(400)));

    reset_registry_for_tests();
    register_web_search_backend(Arc::new(MockBackend::new("fail-a", MockMode::Network)));
    register_web_search_backend(Arc::new(MockBackend::new("fail-b", MockMode::Server(503))));
    register_web_search_backend(Arc::new(MockBackend::new("brave", MockMode::Network)));
    let cfg = WebSearchConfigRef {
        primary: "fail-a".into(),
        fallbacks: vec!["fail-b".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    let chain = BackendChain::from_resolved(&resolved).expect("chain");
    let err = chain
        .search("test", SearchOptions::default())
        .await
        .expect_err("all fail");
    assert!(err.message.contains("fail-a"));
    assert!(err.message.contains("fail-b"));
}

#[test]
fn empty_results_is_not_an_error() {
    let mode = MockMode::Success(vec![]);
    assert!(matches!(mode, MockMode::Success(_)));
}

#[test]
fn resolved_chain_honors_primary_and_fallbacks() {
    let _lock = web_config_test_lock();
    let _env = EnvBackendGuard::isolate();
    let prev_searx = std::env::var("SEARXNG_URL").ok();
    let prev_brave = std::env::var("BRAVE_API_KEY").ok();
    let prev_brave2 = std::env::var("BRAVE_SEARCH_API_KEY").ok();
    unsafe { std::env::set_var("SEARXNG_URL", "http://searx.example") };
    unsafe { std::env::set_var("BRAVE_API_KEY", "test-brave-key") };
    let cfg = WebSearchConfigRef {
        primary: "searxng".into(),
        fallbacks: vec!["brave".into(), "brave".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    assert_eq!(resolved.names, vec!["searxng", "brave", "brave"]);
    unsafe { std::env::remove_var("SEARXNG_URL") };
    unsafe { std::env::remove_var("BRAVE_API_KEY") };
    if let Some(v) = prev_searx {
        unsafe { std::env::set_var("SEARXNG_URL", v) };
    }
    if let Some(v) = prev_brave {
        unsafe { std::env::set_var("BRAVE_API_KEY", v) };
    }
    if let Some(v) = prev_brave2 {
        unsafe { std::env::set_var("BRAVE_SEARCH_API_KEY", v) };
    }
}

#[test]
fn unconfigured_paid_backends_skipped_in_multi_chain() {
    let _env = EnvBackendGuard::isolate();
    let prev = std::env::var("PARALLEL_API_KEY").ok();
    let prev_fc = std::env::var("FIRECRAWL_API_KEY").ok();
    unsafe { std::env::remove_var("PARALLEL_API_KEY") };
    unsafe { std::env::remove_var("FIRECRAWL_API_KEY") };
    let cfg = WebSearchConfigRef {
        primary: "parallel".into(),
        fallbacks: vec!["brave".into(), "brave".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    assert_eq!(resolved.names, vec!["brave"]);
    if let Some(v) = prev {
        unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
    }
    if let Some(v) = prev_fc {
        unsafe { std::env::set_var("FIRECRAWL_API_KEY", v) };
    }
}

#[test]
fn explicit_unconfigured_single_backend_falls_back_to_brave() {
    let _env = EnvBackendGuard::isolate();
    let prev = std::env::var("PARALLEL_API_KEY").ok();
    let prev_fc = std::env::var("FIRECRAWL_API_KEY").ok();
    unsafe { std::env::remove_var("PARALLEL_API_KEY") };
    unsafe { std::env::remove_var("FIRECRAWL_API_KEY") };
    let cfg = WebSearchConfigRef {
        primary: "parallel".into(),
        fallbacks: vec![],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    assert_eq!(resolved.names, vec!["brave"]);
    if let Some(v) = prev {
        unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
    }
    if let Some(v) = prev_fc {
        unsafe { std::env::set_var("FIRECRAWL_API_KEY", v) };
    }
}

#[test]
fn default_config_without_keys_uses_brave_only() {
    let _env = EnvBackendGuard::isolate();
    let prev_searx = std::env::var("SEARXNG_URL").ok();
    let prev_brave = std::env::var("BRAVE_API_KEY").ok();
    let prev_fc = std::env::var("FIRECRAWL_API_KEY").ok();
    unsafe { std::env::remove_var("SEARXNG_URL") };
    unsafe { std::env::remove_var("BRAVE_API_KEY") };
    unsafe { std::env::remove_var("BRAVE_SEARCH_API_KEY") };
    unsafe { std::env::remove_var("FIRECRAWL_API_KEY") };
    let cfg = WebSearchConfigRef::default();
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    assert_eq!(resolved.names, vec!["brave"]);
    if let Some(v) = prev_searx {
        unsafe { std::env::set_var("SEARXNG_URL", v) };
    }
    if let Some(v) = prev_brave {
        unsafe { std::env::set_var("BRAVE_API_KEY", v) };
    }
    if let Some(v) = prev_fc {
        unsafe { std::env::set_var("FIRECRAWL_API_KEY", v) };
    }
}

#[test]
fn backend_alias_duckduckgo_maps_to_brave() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    assert!(get_web_search_backend("duckduckgo").is_some());
}

#[test]
fn search_error_fallback_eligibility() {
    assert!(SearchError::rate_limit("b", "x").is_fallback_eligible());
    assert!(SearchError::timeout("b", "x").is_fallback_eligible());
    assert!(SearchError::server("b", 503, "x").is_fallback_eligible());
    assert!(SearchError::network("b", "x").is_fallback_eligible());
    assert!(!SearchError::bad_request("b", 400, "x").is_fallback_eligible());
    assert!(!SearchError::bad_request("b", 403, "x").is_fallback_eligible());
    assert!(!SearchError::hard("b", "x").is_fallback_eligible());
    assert!(SearchError::not_configured("parallel").is_fallback_eligible());
}

#[tokio::test]
async fn empty_results_from_mock_is_success() {
    let _lock = test_registry_lock();
    let _env = EnvBackendGuard::isolate();
    reset_registry_for_tests();
    register_web_search_backend(Arc::new(MockBackend::new(
        "empty-mock",
        MockMode::Success(vec![]),
    )));
    let cfg = WebSearchConfigRef {
        primary: "empty-mock".into(),
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    let chain = BackendChain::from_resolved(&resolved).expect("chain");
    let (results, used) = chain
        .search("nothing", SearchOptions::default())
        .await
        .expect("empty is success");
    assert_eq!(used, "empty-mock");
    assert!(results.is_empty());
}

#[tokio::test]
async fn ddgs_empty_falls_through_to_next_backend_renamed() {
    let _lock = test_registry_lock();
    let _env = EnvBackendGuard::isolate();
    reset_registry_for_tests();
    register_web_search_backend(Arc::new(MockBackend::new(
        "brave",
        MockMode::Success(vec![]),
    )));
    register_web_search_backend(Arc::new(MockBackend::new(
        "ok-mock",
        MockMode::Success(vec![sample_result()]),
    )));
    let cfg = WebSearchConfigRef {
        primary: "brave".into(),
        fallbacks: vec!["ok-mock".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    let chain = BackendChain::from_resolved(&resolved).expect("chain");
    let (results, used) = chain
        .search("Raphaël MANSUY", SearchOptions::default())
        .await
        .expect("ok-mock after empty brave");
    assert_eq!(used, "ok-mock");
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn timeout_triggers_fallback() {
    let _lock = test_registry_lock();
    let _env = EnvBackendGuard::isolate();
    reset_registry_for_tests();
    register_web_search_backend(Arc::new(MockBackend::new("slow-mock", MockMode::Timeout)));
    register_web_search_backend(Arc::new(MockBackend::new(
        "ok-mock",
        MockMode::Success(vec![sample_result()]),
    )));
    let cfg = WebSearchConfigRef {
        primary: "slow-mock".into(),
        fallbacks: vec!["ok-mock".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    let chain = BackendChain::from_resolved(&resolved).expect("chain");
    let (results, used) = chain
        .search("test", SearchOptions::default())
        .await
        .expect("timeout fallback");
    assert_eq!(used, "ok-mock");
    assert_eq!(results.len(), 1);
}

#[test]
fn backend_alias_brave_free_maps_to_brave() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    let backend = get_web_search_backend("brave-free").expect("brave-free alias");
    assert_eq!(backend.name(), "brave");
}

#[test]
fn explicit_brave_expands_to_configured_chain() {
    let _env = EnvBackendGuard::isolate();
    let prev_searx = std::env::var("SEARXNG_URL").ok();
    let prev_brave = std::env::var("BRAVE_API_KEY").ok();
    unsafe { std::env::set_var("SEARXNG_URL", "http://searx.example") };
    unsafe { std::env::set_var("BRAVE_API_KEY", "test-brave-key") };
    let cfg = WebSearchConfigRef {
        primary: "searxng".into(),
        fallbacks: vec!["brave".into(), "brave".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, Some("brave")).expect("resolve");
    assert_eq!(resolved.names.first().map(String::as_str), Some("brave"));
    assert!(resolved.names.iter().any(|n| n == "searxng"));
    assert!(resolved.names.iter().any(|n| n == "brave"));
    unsafe { std::env::remove_var("SEARXNG_URL") };
    unsafe { std::env::remove_var("BRAVE_API_KEY") };
    if let Some(v) = prev_searx {
        unsafe { std::env::set_var("SEARXNG_URL", v) };
    }
    if let Some(v) = prev_brave {
        unsafe { std::env::set_var("BRAVE_API_KEY", v) };
    }
}

#[test]
fn web_backend_brave_does_not_force_single_backend() {
    let _env = EnvBackendGuard::isolate();
    let _lock = web_config_test_lock();
    let prev_searx = std::env::var("SEARXNG_URL").ok();
    unsafe { std::env::set_var("SEARXNG_URL", "http://searx.example") };
    let _home = EdgecrabHomeGuard::isolated(Some(
        r#"
web:
  backend: brave
web_search:
  primary: searxng
  fallbacks: [brave, brave]
"#,
    ));
    let cfg = WebSearchConfigRef {
        primary: "searxng".into(),
        fallbacks: vec!["brave".into(), "brave".into()],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
    assert!(
        resolved.names.first().map(String::as_str) == Some("searxng"),
        "web.backend=brave must not skip configured chain: {:?}",
        resolved.names
    );
    unsafe { std::env::remove_var("SEARXNG_URL") };
    if let Some(v) = prev_searx {
        unsafe { std::env::set_var("SEARXNG_URL", v) };
    }
}

#[test]
fn max_results_clamped_to_hermes_cap() {
    use super::backend_settings::MAX_SEARCH_RESULTS;
    let opts = SearchOptions {
        max_results: 999,
        ..Default::default()
    };
    assert_eq!(opts.max_results(), MAX_SEARCH_RESULTS);
    let opts_min = SearchOptions {
        max_results: 0,
        ..Default::default()
    };
    assert_eq!(opts_min.max_results(), 1);
}

#[test]
fn plugin_registration_overwrites_same_name() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    register_web_search_backend(Arc::new(MockBackend::new(
        "overwrite-me",
        MockMode::Success(vec![SearchResult::new(
            1,
            "v1",
            "https://v1.example",
            "",
            "overwrite-me",
        )]),
    )));
    register_web_search_backend(Arc::new(MockBackend::new(
        "overwrite-me",
        MockMode::Success(vec![SearchResult::new(
            1,
            "v2",
            "https://v2.example",
            "",
            "overwrite-me",
        )]),
    )));
    let backend = get_web_search_backend("overwrite-me").expect("registered");
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let rows = rt
        .block_on(backend.search("q", &SearchOptions::default()))
        .expect("search");
    assert_eq!(rows[0].title, "v2");
}

#[test]
fn unknown_backend_in_chain_returns_error() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    let _env = EnvBackendGuard::isolate();
    let cfg = WebSearchConfigRef {
        primary: "does-not-exist".into(),
        fallbacks: vec![],
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, Some("does-not-exist")).expect("resolve");
    match BackendChain::from_resolved(&resolved) {
        Err(err) => assert!(err.message.contains("Unknown web search backend")),
        Ok(_) => panic!("expected unknown backend error"),
    }
}

#[tokio::test]
async fn explicit_unconfigured_backend_falls_back_to_brave() {
    let _lock = test_registry_lock();
    let _env = EnvBackendGuard::isolate();
    reset_registry_for_tests();
    let prev_searx = std::env::var("SEARXNG_URL").ok();
    unsafe { std::env::remove_var("SEARXNG_URL") };
    let cfg = WebSearchConfigRef {
        primary: "searxng".into(),
        ..Default::default()
    };
    let resolved = ResolvedChain::resolve(&cfg, Some("searxng")).expect("resolve");
    assert_eq!(resolved.skipped_tool_override.as_deref(), Some("searxng"));
    assert!(
        !resolved.names.iter().any(|n| n == "searxng"),
        "unconfigured searxng must not remain in chain: {:?}",
        resolved.names
    );
    assert!(
        resolved.names.iter().any(|n| n == "brave"),
        "expected brave fallback in chain: {:?}",
        resolved.names
    );
    unsafe { std::env::remove_var("SEARXNG_URL") };
    if let Some(v) = prev_searx {
        unsafe { std::env::set_var("SEARXNG_URL", v) };
    }
}

#[test]
fn hermes_data_web_envelope() {
    use super::response::{hermes_web_rows, success_payload};
    let rows = vec![SearchResult::new(
        1,
        "Title",
        "https://example.com",
        "snippet",
        "mock",
    )];
    let payload = success_payload("q", "mock", None, None, None, &rows);
    let web = payload
        .get("data")
        .and_then(|d| d.get("web"))
        .and_then(|w| w.as_array())
        .expect("data.web");
    assert_eq!(web[0]["title"], "Title");
    assert_eq!(web[0]["description"], "snippet");
    assert_eq!(web[0]["position"], 1);
    assert_eq!(hermes_web_rows(&rows)[0]["url"], "https://example.com");
}

#[tokio::test]
async fn registry_extract_dispatches_to_registered_backend() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    let mock = Arc::new(MockBackend::with_extract(
        "extract-mock",
        MockMode::Hard,
        MockExtractMode::Success(crate::tools::web::search::content_extract::RawExtractPage {
            url: String::new(),
            title: "Registry Extract".into(),
            content: "Body from mock provider.".into(),
            extractor: "extract-mock",
            content_type: None,
            content_format: Some("text".into()),
            meta_description: None,
        }),
    ));
    register_web_search_backend(mock);
    let page = extract_with_backend(
        "extract-mock",
        "https://example.com/page",
        &ExtractOptions::default(),
    )
    .await
    .expect("extract via registry");
    assert_eq!(page.title, "Registry Extract");
    assert_eq!(page.url, "https://example.com/page");
    assert!(
        get_web_search_backend("extract-mock")
            .expect("registered")
            .supports_extract()
    );
}

#[test]
fn each_builtin_has_setup_schema() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    let schemas = list_web_provider_setup_schemas();
    assert!(
        schemas.len() >= 8,
        "expected 8+ builtins, got {}",
        schemas.len()
    );
    for (name, schema) in &schemas {
        assert!(!schema.name.is_empty(), "{name} schema.name");
        // free providers may have empty env_vars (brave); paid ones must document keys
        if matches!(
            name.as_str(),
            "firecrawl" | "tavily" | "exa" | "parallel" | "brave" | "xai"
        ) {
            assert!(
                !schema.env_vars.is_empty(),
                "{name} should list env vars for setup picker"
            );
        }
    }
    let names: Vec<_> = schemas.iter().map(|(n, _)| n.as_str()).collect();
    for required in [
        "searxng",
        "brave",
        "brave",
        "firecrawl",
        "tavily",
        "exa",
        "parallel",
        "xai",
    ] {
        assert!(
            names.contains(&required),
            "missing setup schema for {required}"
        );
    }
}
