#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: Hermes-style registry dispatch for `web_extract` paid backends.

mod common;

use std::sync::Arc;

use common::test_ctx;
use edgecrab_tools::ToolHandler;
use edgecrab_tools::tools::web::WebExtractTool;
use edgecrab_tools::tools::web::search::backends::mock::{MockBackend, MockExtractMode, MockMode};
use edgecrab_tools::tools::web::search::content_extract::RawExtractPage;
use edgecrab_tools::tools::web::search::registry::{
    register_web_search_backend, reset_registry_for_tests, test_registry_lock,
};
use serde_json::json;

#[tokio::test]
async fn e2e_extract_uses_registry_when_env_points_at_plugin_backend() {
    let _lock = test_registry_lock();
    reset_registry_for_tests();
    register_web_search_backend(Arc::new(MockBackend::with_extract(
        "extract-mock",
        MockMode::Hard,
        MockExtractMode::Success(RawExtractPage {
            url: String::new(),
            title: "Plugin Extract".into(),
            content: "Content from registered provider.".into(),
            extractor: "extract-mock",
            content_type: None,
            content_format: Some("markdown".into()),
            meta_description: Some("mock meta".into()),
        }),
    )));

    unsafe { std::env::set_var("EDGECRAB_WEB_EXTRACT_BACKEND", "extract-mock") };

    let result = WebExtractTool
        .execute(json!({"url": "https://example.com/docs"}), &test_ctx())
        .await
        .expect("registry-backed extract");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["backend"], "extract-mock");
    assert_eq!(parsed["result"]["title"], "Plugin Extract");
    assert!(
        parsed["result"]["content"]
            .as_str()
            .unwrap_or("")
            .contains("registered provider")
    );

    unsafe { std::env::remove_var("EDGECRAB_WEB_EXTRACT_BACKEND") };
    reset_registry_for_tests();
}
