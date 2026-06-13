#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: Exa / Parallel extract backend fail-fast and auto-chain ordering.

mod common;

use common::test_ctx;
use edgecrab_tools::ToolHandler;
use edgecrab_tools::tools::web::WebExtractTool;
use serde_json::json;

#[tokio::test]
async fn e2e_extract_exa_explicit_unconfigured_fail_fast() {
    let prev = std::env::var("EXA_API_KEY").ok();
    unsafe { std::env::remove_var("EXA_API_KEY") };
    let err = WebExtractTool
        .execute(
            json!({"url": "https://www.rust-lang.org/", "backend": "exa"}),
            &test_ctx(),
        )
        .await
        .expect_err("exa extract without key");
    assert!(err.to_string().contains("EXA_API_KEY"));
    if let Some(v) = prev {
        unsafe { std::env::set_var("EXA_API_KEY", v) };
    }
}

#[tokio::test]
async fn e2e_extract_parallel_explicit_unconfigured_fail_fast() {
    let prev = std::env::var("PARALLEL_API_KEY").ok();
    unsafe { std::env::remove_var("PARALLEL_API_KEY") };
    let err = WebExtractTool
        .execute(
            json!({"url": "https://www.rust-lang.org/", "backend": "parallel"}),
            &test_ctx(),
        )
        .await
        .expect_err("parallel extract without key");
    assert!(err.to_string().contains("PARALLEL_API_KEY"));
    if let Some(v) = prev {
        unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
    }
}

#[tokio::test]
#[ignore = "requires EXA_API_KEY"]
async fn e2e_extract_exa_when_key_set() {
    if std::env::var("EXA_API_KEY").is_err() {
        eprintln!("Skipping: EXA_API_KEY not set");
        return;
    }
    let result = WebExtractTool
        .execute(
            json!({
                "url": "https://www.rust-lang.org/",
                "backend": "exa",
                "max_chars": 2000
            }),
            &test_ctx(),
        )
        .await
        .expect("exa extract");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["extractor"], "exa");
    assert!(
        parsed["content"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("rust")
    );
}

#[tokio::test]
#[ignore = "requires PARALLEL_API_KEY"]
async fn e2e_extract_parallel_when_key_set() {
    if std::env::var("PARALLEL_API_KEY").is_err() {
        eprintln!("Skipping: PARALLEL_API_KEY not set");
        return;
    }
    let result = WebExtractTool
        .execute(
            json!({
                "url": "https://www.rust-lang.org/",
                "backend": "parallel",
                "max_chars": 2000
            }),
            &test_ctx(),
        )
        .await
        .expect("parallel extract");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["extractor"], "parallel");
    assert!(!parsed["content"].as_str().unwrap_or("").is_empty());
}
