//! DDGS edge cases — Bing false-positive, HTTP 202 bot-challenge, URL decode, engine parsers.

mod common;

use common::registry_guard;
use edgecrab_tools::tools::web::search::backends::ddgs::{
    engine_reports_no_results, filter_relevant, is_bot_challenge, is_engine_blocked,
    normalize_bing_url, normalize_ddg_url, parse_bing_html, parse_ddg_lite, parse_engine_html,
    DdgsEngine,
};

#[test]
fn e2e_normalize_bing_url_decodes_amp_entities_and_a1_prefix() {
    let _lock = registry_guard();
    let raw = "https://www.bing.com/ck/a?!&amp;&amp;p=x&amp;u=a1aHR0cHM6Ly93d3cuZXhhbXBsZS5vcmcv&amp;ntb=1";
    assert_eq!(
        normalize_bing_url(raw).as_deref(),
        Some("https://www.example.org/")
    );
}

#[test]
fn e2e_normalize_bing_url_passthrough_direct_https() {
    let _lock = registry_guard();
    assert_eq!(
        normalize_bing_url("https://rust-lang.org/").as_deref(),
        Some("https://rust-lang.org/")
    );
}

#[test]
fn e2e_normalize_ddg_lite_redirect() {
    let _lock = registry_guard();
    let raw = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage";
    assert_eq!(
        normalize_ddg_url(raw).as_deref(),
        Some("https://example.com/page")
    );
}

#[test]
fn e2e_bing_turnstile_js_does_not_block_when_algo_present() {
    let _lock = registry_guard();
    let html = r#"
        <script>turnstile challenge widget</script>
        <li class="b_algo">
            <h2 class=""><a href="https://example.org/">Example</a></h2>
            <p>snippet</p>
        </li>
    "#;
    assert!(!is_engine_blocked(html));
    let results = parse_bing_html(html, 3, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
}

#[test]
fn e2e_ddg_lite_table_row_parser() {
    let _lock = registry_guard();
    let html = r#"
        <table>
            <tr><td><a href="https://example.net/">Example Net</a></td></tr>
            <tr><td class='result-snippet'>A useful snippet.</td></tr>
        </table>
    "#;
    let results = parse_ddg_lite(html, 5, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://example.net/");
    assert!(results[0].snippet.contains("useful"));
}

#[test]
fn e2e_bing_h2_class_attribute_serp_block() {
    let _lock = registry_guard();
    let html = r#"
        <li class="b_algo" data-id="">
            <h2 class=""><a href="https://www.bing.com/ck/a?!&amp;&amp;p=x&amp;u=a1aHR0cHM6Ly9ydXN0LWxhbmcub3JnLw&amp;ntb=1"><strong>Rust</strong></a></h2>
            <div class="b_caption"><p class="b_lineclamp2">Systems language.</p></div>
        </li>
    "#;
    let results = parse_bing_html(html, 3, "ddgs").expect("parse");
    assert_eq!(results[0].url, "https://rust-lang.org/");
}

#[test]
fn e2e_bing_parses_despite_embedded_js_no_results_string() {
    let _lock = registry_guard();
    let html = r#"
        <script>"There are no results for this question, please check your spelling"</script>
        <li class="b_algo">
            <h2><a href="https://www.rust-lang.org/">Rust Programming Language</a></h2>
            <p>A language empowering everyone to build reliable software.</p>
        </li>
    "#;
    assert!(!engine_reports_no_results(DdgsEngine::Bing, html));
    let results = parse_bing_html(html, 5, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
    assert!(results[0].url.contains("rust-lang.org"));
}

#[test]
fn e2e_ddg_http202_anomaly_body_detected_as_bot_challenge() {
    let _lock = registry_guard();
    let html = r#"<html><body><div class="anomaly-modal__title">Bots use DuckDuckGo too.</div></body></html>"#;
    assert!(is_bot_challenge(html));
    assert!(is_engine_blocked(html));
}

#[test]
fn e2e_ddg_html_div_h2_parser_matches_python_xpath() {
    let _lock = registry_guard();
    let html = r#"
        <div>
            <a href="https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com">
                <h2><a>Raphaël (peintre)</a></h2>
                <a>Peintre italien de la Renaissance.</a>
            </a>
        </div>
    "#;
    let results = parse_engine_html(DdgsEngine::Html, html, 3, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://example.com");
}

#[test]
fn e2e_bing_ck_href_decodes_to_destination_url() {
    let _lock = registry_guard();
    let html = r#"
        <li class="b_algo">
            <h2 class=""><a href="https://www.bing.com/ck/a?!&amp;&amp;p=x&amp;u=a1aHR0cHM6Ly93d3cuZXhhbXBsZS5vcmcv&amp;ntb=1">Example</a></h2>
            <p>snippet</p>
        </li>
    "#;
    let results = parse_bing_html(html, 3, "ddgs").expect("parse");
    assert_eq!(results[0].url, "https://www.example.org/");
    assert!(!results[0].url.contains("bing.com"));
}

#[test]
fn e2e_bing_empty_page_without_algo_is_success_shape() {
    let _lock = registry_guard();
    let html = r#"<html><body><script>"There are no results for"</script></body></html>"#;
    assert!(!engine_reports_no_results(DdgsEngine::Bing, html));
    let results = parse_bing_html(html, 5, "ddgs").expect("parse");
    assert!(results.is_empty());
}

#[test]
fn e2e_bing_snippet_strips_css_noise() {
    let _lock = registry_guard();
    let html = r#"
        <li class="b_algo">
            <h2><a href="https://example.org/">Example</a></h2>
            <div class="b_caption"><p class="b_lineclamp2">Real summary about the topic.</p>
            <style>.b_imgcap{display:flex;flex-direction:row-reverse}</style></div>
        </li>
    "#;
    let results = parse_bing_html(html, 3, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
    assert!(results[0].snippet.contains("Real summary"));
    assert!(!results[0].snippet.contains("flex-direction"));
}

#[test]
fn e2e_relevance_rejects_poisoned_bing_serp_for_person_query() {
    let _lock = registry_guard();
    let query = "Raphaël MANSUY";
    let html = r#"
        <li class="b_algo"><h2><a href="https://dexsport.io/">Dexsport crypto betting</a></h2><p>Web3 sportsbook</p></li>
        <li class="b_algo"><h2><a href="https://example.com/dex">Dexsport Review</a></h2><p>No KYC</p></li>
        <li class="b_algo"><h2><a href="https://cryptoslate.com/dex">Dexsport Price</a></h2><p>DESU token</p></li>
    "#;
    let parsed = parse_bing_html(html, 5, "ddgs").expect("parse");
    assert_eq!(parsed.len(), 3, "parser should surface raw Bing blocks");
    assert!(
        filter_relevant(query, parsed).is_empty(),
        "poisoned SERP must not reach the agent as relevant hits"
    );
}
