#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: web provider diagnostics for doctor / setup picker parity.

mod common;

use common::registry_guard;
use edgecrab_tools::{
    collect_web_diagnostics, format_extract_doctor_detail, format_search_doctor_detail,
    web_provider_picker_rows,
};

#[test]
fn e2e_doctor_web_search_always_ready_via_ddgs() {
    let _lock = registry_guard();
    let report = collect_web_diagnostics();
    assert!(
        report.search_ready,
        "ddgs fallback should keep search_ready true"
    );
    let detail = format_search_doctor_detail(&report);
    assert!(
        detail.contains("ddgs") || detail.contains("ready"),
        "unexpected search detail: {detail}"
    );
}

#[test]
fn e2e_picker_rows_serializable_and_complete() {
    let _lock = registry_guard();
    let rows = web_provider_picker_rows();
    assert!(rows.len() >= 8);
    let json = serde_json::to_string(&rows).expect("serialize picker rows");
    assert!(json.contains("firecrawl"));
    assert!(json.contains("supports_crawl"));
}

#[test]
fn e2e_extract_doctor_mentions_native_fallback() {
    let _lock = registry_guard();
    let report = collect_web_diagnostics();
    let detail = format_extract_doctor_detail(&report);
    assert!(
        detail.contains("native") || detail.contains("configured"),
        "extract detail should mention native or configured backend: {detail}"
    );
}
