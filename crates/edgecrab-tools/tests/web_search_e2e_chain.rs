#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: `web_search` primary / fallback chain persistence and diagnostics.

mod common;

use common::registry_guard;
use edgecrab_tools::{
    WebSearchChainUpdate, clear_web_search_chain_in_config, collect_web_diagnostics,
    format_search_chain_summary, load_web_search_config_from_path,
    persist_web_search_chain_in_config,
};
use tempfile::TempDir;

#[test]
fn e2e_persist_search_chain_primary_and_fallbacks() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("config.yaml");
    persist_web_search_chain_in_config(
        &path,
        &WebSearchChainUpdate {
            primary: Some("searxng".into()),
            fallbacks: Some(vec!["brave".into(), "ddgs".into()]),
            timeout_secs: Some(15),
        },
    )
    .expect("persist");
    let cfg = load_web_search_config_from_path(&path).expect("parse");
    assert_eq!(cfg.primary, "searxng");
    assert_eq!(cfg.fallbacks, vec!["brave", "ddgs"]);
    assert_eq!(cfg.timeout_secs, 15);
    // Summary reflects the chain that will actually run (unconfigured paid backends stripped).
    let summary = format_search_chain_summary(&cfg);
    assert!(
        summary.contains("ddgs") && summary.contains("15s timeout"),
        "unexpected summary: {summary}"
    );
}

#[test]
fn e2e_clear_search_chain_resets_to_auto_summary() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("config.yaml");
    persist_web_search_chain_in_config(
        &path,
        &WebSearchChainUpdate {
            primary: Some("brave".into()),
            fallbacks: Some(vec!["ddgs".into()]),
            timeout_secs: None,
        },
    )
    .expect("persist");
    clear_web_search_chain_in_config(&path).expect("clear");
    let cfg = load_web_search_config_from_path(&path).expect("parse");
    assert!(cfg.primary.is_empty());
    assert!(cfg.fallbacks.is_empty());
    // Empty config → auto-resolved chain (ddgs when no paid backends configured).
    let summary = format_search_chain_summary(&cfg);
    assert!(
        summary.starts_with("ddgs") && summary.contains("timeout"),
        "unexpected auto summary: {summary}"
    );
}

#[test]
fn e2e_diagnostics_report_includes_chain_summary() {
    let _lock = registry_guard();
    let report = collect_web_diagnostics();
    let text = edgecrab_tools::format_web_setup_report(&report);
    assert!(text.contains("Chain:"));
    assert!(!report.search_chain_summary.is_empty());
}
