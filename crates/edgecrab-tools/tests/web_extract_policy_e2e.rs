#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: website blocklist policy gates `web_extract`.

mod common;

use common::test_ctx;
use edgecrab_security::website_policy::invalidate_cache;
use edgecrab_tools::ToolHandler;
use edgecrab_tools::tools::web::WebExtractTool;
use serde_json::json;
use std::sync::Mutex;
use tempfile::TempDir;

static POLICY_E2E_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn e2e_web_extract_blocked_by_website_policy() {
    let _lock = POLICY_E2E_LOCK.lock().expect("policy e2e lock");
    invalidate_cache();
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        r#"
security:
  website_blocklist:
    enabled: true
    domains: [evil.test]
"#,
    )
    .expect("write config");
    unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };

    let err = WebExtractTool
        .execute(json!({"url": "https://www.evil.test/secret"}), &test_ctx())
        .await
        .expect_err("blocked extract");
    assert!(err.to_string().contains("website policy"));

    unsafe { std::env::remove_var("EDGECRAB_HOME") };
    invalidate_cache();
}

#[tokio::test]
async fn e2e_web_extract_allowed_when_policy_disabled() {
    let _lock = POLICY_E2E_LOCK.lock().expect("policy e2e lock");
    invalidate_cache();
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("config.yaml"),
        r#"
security:
  website_blocklist:
    enabled: false
    domains: [evil.test]
"#,
    )
    .expect("write config");
    unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };

    let result = WebExtractTool
        .execute(json!({"url": "https://www.rust-lang.org/"}), &test_ctx())
        .await;
    if let Err(err) = result {
        assert!(
            !err.to_string().contains("website policy"),
            "unexpected policy block: {err}"
        );
    }

    unsafe { std::env::remove_var("EDGECRAB_HOME") };
    invalidate_cache();
}
