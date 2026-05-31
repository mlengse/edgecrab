//! E2E: Hermes `web.extract_backend` config drives web_extract routing.

mod common;

use common::test_ctx;
use edgecrab_tools::ToolHandler;
use edgecrab_tools::tools::web::WebExtractTool;
use serde_json::json;
use std::sync::Mutex;
use tempfile::TempDir;

static EXTRACT_CONFIG_E2E_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn e2e_config_extract_backend_falls_through_to_native_when_unconfigured() {
    let _lock = EXTRACT_CONFIG_E2E_LOCK
        .lock()
        .expect("extract config e2e lock");
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        r#"
web:
  extract_backend: exa
"#,
    )
    .expect("write config");
    unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };

    let keys = [
        "EXA_API_KEY",
        "PARALLEL_API_KEY",
        "FIRECRAWL_API_KEY",
        "TAVILY_API_KEY",
    ];
    let saved: Vec<_> = keys.iter().map(|k| (*k, std::env::var(k).ok())).collect();
    for key in keys {
        unsafe { std::env::remove_var(key) };
    }
    unsafe { std::env::remove_var("EDGECRAB_WEB_EXTRACT_BACKEND") };

    let result = WebExtractTool
        .execute(json!({"url": "https://rust-lang.org/"}), &test_ctx())
        .await
        .expect("native fallback extract");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    // Config asked for exa but key missing → auto chain → native
    assert_eq!(parsed["backend"], "native");

    unsafe { std::env::remove_var("EDGECRAB_HOME") };
    for (key, val) in saved {
        unsafe { std::env::remove_var(key) };
        if let Some(v) = val {
            unsafe { std::env::set_var(key, v) };
        }
    }
}

#[tokio::test]
async fn e2e_config_extract_backend_brave_falls_through_to_native() {
    let _lock = EXTRACT_CONFIG_E2E_LOCK
        .lock()
        .expect("extract config e2e lock");
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        r#"
web_search:
  backends:
    brave:
      api_key: test-brave-key
web:
  extract_backend: brave
"#,
    )
    .expect("write config");
    unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };
    unsafe { std::env::remove_var("EDGECRAB_WEB_EXTRACT_BACKEND") };

    let keys = [
        "EXA_API_KEY",
        "PARALLEL_API_KEY",
        "FIRECRAWL_API_KEY",
        "TAVILY_API_KEY",
    ];
    let saved: Vec<_> = keys.iter().map(|k| (*k, std::env::var(k).ok())).collect();
    for key in keys {
        unsafe { std::env::remove_var(key) };
    }

    let result = WebExtractTool
        .execute(json!({"url": "https://rust-lang.org/"}), &test_ctx())
        .await
        .expect("search-only config should fall through to native");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["backend"], "native");

    unsafe { std::env::remove_var("EDGECRAB_HOME") };
    for (key, val) in saved {
        unsafe { std::env::remove_var(key) };
        if let Some(v) = val {
            unsafe { std::env::set_var(key, v) };
        }
    }
}

#[tokio::test]
async fn e2e_explicit_ddgs_extract_returns_search_only_error() {
    let _lock = EXTRACT_CONFIG_E2E_LOCK
        .lock()
        .expect("extract config e2e lock");
    unsafe { std::env::remove_var("EDGECRAB_HOME") };
    unsafe { std::env::set_var("EDGECRAB_WEB_EXTRACT_BACKEND", "ddgs") };

    let err = WebExtractTool
        .execute(json!({"url": "https://rust-lang.org/"}), &test_ctx())
        .await
        .expect_err("ddgs is search-only for extract");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("search-only"),
        "expected search-only error, got: {err}"
    );

    unsafe { std::env::remove_var("EDGECRAB_WEB_EXTRACT_BACKEND") };
}
