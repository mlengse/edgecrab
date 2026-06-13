#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E/unit integration: DDGS module exceeds Hermes ddgs provider contract.

mod common;

use common::registry_guard;
use edgecrab_tools::tools::web::search::backends::ddgs::{
    normalize_ddg_url, parse_bing_html, parse_ddg_html,
};
use edgecrab_tools::tools::web::search::get_web_search_backend;

#[test]
fn e2e_ddgs_backend_display_name_and_always_available() {
    let _lock = registry_guard();
    let backend = get_web_search_backend("ddgs").expect("ddgs registered");
    assert_eq!(backend.display_name(), "DuckDuckGo (ddgs)");
    assert!(backend.is_available());
    assert!(backend.supports_search());
    assert!(!backend.supports_extract());
}

#[test]
fn e2e_ddgs_empty_results_is_success_shape() {
    let _lock = registry_guard();
    let results = parse_ddg_html("<html><body></body></html>", 5, "ddgs").expect("parse");
    assert!(results.is_empty());
}

#[test]
fn e2e_ddgs_redirect_url_matches_hermes_href_fallback() {
    let _lock = registry_guard();
    let html = r#"
        <a class="result__a" href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2F">Rust Lang</a>
        <a class="result__snippet">The Rust programming language</a>
    "#;
    let results = parse_ddg_html(html, 5, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://www.rust-lang.org/");
    assert_eq!(results[0].title, "Rust Lang");
    assert_eq!(
        normalize_ddg_url("https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com").as_deref(),
        Some("https://example.com")
    );
}

#[test]
fn e2e_ddgs_respects_max_results_cap() {
    let _lock = registry_guard();
    let html = r#"
        <a class="result__a" href="https://a.example">A</a>
        <a class="result__a" href="https://b.example">B</a>
        <a class="result__a" href="https://c.example">C</a>
    "#;
    let results = parse_ddg_html(html, 2, "ddgs").expect("parse");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].rank, 1);
    assert_eq!(results[1].rank, 2);
}

#[test]
fn e2e_ddgs_result_shape_has_required_fields() {
    let _lock = registry_guard();
    let html = r#"
        <li class="b_algo">
            <h2><a href="https://example.com/doc">Title</a></h2>
            <p>Snippet body.</p>
        </li>
    "#;
    let results = parse_bing_html(html, 1, "ddgs").expect("parse");
    let hit = &results[0];
    assert_eq!(hit.rank, 1);
    assert_eq!(hit.title, "Title");
    assert_eq!(hit.url, "https://example.com/doc");
    assert!(hit.snippet.contains("Snippet"));
    assert_eq!(hit.source, "ddgs");
}
