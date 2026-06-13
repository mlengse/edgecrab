#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! DDGS selection E2E — real-world queries + edge cases (fixture HTML, no network).

mod common;

use common::registry_guard;
use edgecrab_tools::tools::web::search::backends::ddgs::{
    extend_pool, parse_bing_html, select_ranked, select_raw,
};

fn bing_li(title: &str, url: &str, snippet: &str) -> String {
    format!(r#"<li class="b_algo"><h2><a href="{url}">{title}</a></h2><p>{snippet}</p></li>"#)
}

fn parse_bing_blocks(
    blocks: &str,
    max: usize,
) -> Vec<edgecrab_tools::tools::web::search::backend::SearchResult> {
    parse_bing_html(blocks, max, "ddgs").expect("bing parse")
}

fn select_ranked_query(
    query: &str,
    html: &str,
    max: usize,
) -> Vec<edgecrab_tools::tools::web::search::backend::SearchResult> {
    select_ranked(query, parse_bing_blocks(html, max), max)
}

fn select_raw_html(
    html: &str,
    max: usize,
) -> Vec<edgecrab_tools::tools::web::search::backend::SearchResult> {
    select_raw(parse_bing_blocks(html, max), max)
}

#[test]
fn e2e_query_person_name_prefers_linkedin_in_ranked_mode() {
    let _lock = registry_guard();
    let html = format!(
        "{}{}{}",
        bing_li("Dexsport Web3", "https://dexsport.io/", "Sports betting"),
        bing_li(
            "Rapha&#235;l MANSUY | LinkedIn",
            "https://www.linkedin.com/in/raphaelmansuy",
            "Data engineering leader"
        ),
        bing_li(
            "Dexsport token",
            "https://coinmarketcap.com/dex",
            "DESU price"
        ),
    );
    let out = select_ranked_query("Raphaël MANSUY", &html, 3);
    assert!(out[0].url.contains("linkedin.com"));
}

#[test]
fn e2e_query_person_poison_raw_returns_all_like_python() {
    let _lock = registry_guard();
    let html = format!(
        "{}{}{}",
        bing_li("Dexsport crypto", "https://dexsport.io/", "Web3"),
        bing_li("Dexsport review", "https://example.com/dex", "No KYC"),
        bing_li("Dexsport price", "https://cryptoslate.com/x", "Token"),
    );
    assert_eq!(select_raw_html(&html, 5).len(), 3);
}

#[test]
fn e2e_query_rust_programming_keeps_serp_order_in_raw_mode() {
    let _lock = registry_guard();
    let html = format!(
        "{}{}",
        bing_li("Book", "https://doc.rust-lang.org/book/", "book"),
        bing_li("Rust Lang", "https://www.rust-lang.org/", "official"),
    );
    let out = select_raw_html(&html, 2);
    assert_eq!(out[0].url, "https://doc.rust-lang.org/book/");
}

#[test]
fn e2e_edge_ddg_ad_url_filtered_on_html_backend_only() {
    let _lock = registry_guard();
    let html = r#"
        <div>
            <a href="https://duckduckgo.com/y.js?ad_domain=spam.example">
                <h2><a>Sponsored</a></h2>
                <a>buy now</a>
            </a>
        </div>
    "#;
    let results =
        edgecrab_tools::tools::web::search::backends::ddgs::parse_ddg_html(html, 5, "ddgs")
            .expect("parse");
    assert!(results.is_empty());
}

#[test]
fn e2e_edge_bing_keeps_ad_url_like_python() {
    let _lock = registry_guard();
    let html = bing_li(
        "Sponsored",
        "https://duckduckgo.com/y.js?ad_domain=spam.example",
        "buy now",
    );
    assert_eq!(select_raw_html(&html, 5).len(), 1);
}

#[test]
fn e2e_edge_merge_pool_preserves_order() {
    let _lock = registry_guard();
    let mut pool = parse_bing_blocks(&bing_li("A", "https://a.example/", "first"), 1);
    extend_pool(
        &mut pool,
        parse_bing_blocks(&bing_li("B", "https://b.example/", "second"), 1),
    );
    assert_eq!(pool.len(), 2);
    assert_eq!(pool[0].url, "https://a.example/");
}
