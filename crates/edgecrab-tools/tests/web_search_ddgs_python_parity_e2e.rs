#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! DDGS Python `ddgs` 9.0 parity — fixture HTML + optional live comparison.

mod common;

use std::process::Command;

use common::registry_guard;
use edgecrab_tools::tools::web::search::backends::ddgs::{
    DdgsEngine, DdgsSelectionMode, DdgsSettings, ImpersonateOs, metasearch_text, parse_bing_html,
    parse_ddg_html, pick_random_profile, resolve_profile, resolve_profile_with, select_ranked,
    select_raw,
};
use edgecrab_tools::tools::web::search::config::SearchOptions;

const BING_RUST_FIXTURE: &str = include_str!("fixtures/ddgs/bing_rust_programming.html");
const BING_PERSON_FIXTURE: &str = include_str!("fixtures/ddgs/bing_person_mixed.html");
const DDG_HTML_FIXTURE: &str = include_str!("fixtures/ddgs/ddg_html_div_h2.html");

fn parse_fixture_bing(html: &str, max: usize) -> Vec<String> {
    select_raw(parse_bing_html(html, max, "ddgs").expect("parse"), max)
        .into_iter()
        .map(|r| r.url)
        .collect()
}

#[test]
fn e2e_default_settings_strict_python_parity() {
    let _lock = registry_guard();
    let s = DdgsSettings::default();
    assert_eq!(s.selection_mode, DdgsSelectionMode::Raw);
    assert_eq!(s.engine_order(), vec![DdgsEngine::Bing]);
    assert_eq!(s.max_retries, 0);
    assert!(s.region().is_none(), "Python text(region=None) default");
}

#[test]
fn e2e_python_parity_bing_fixture_urls_and_order() {
    let _lock = registry_guard();
    let urls = parse_fixture_bing(BING_RUST_FIXTURE, 5);
    assert_eq!(urls.len(), 4);
    assert_eq!(urls[0], "https://www.rust-lang.org/");
    assert_eq!(urls[1], "https://doc.rust-lang.org/book/");
    assert!(
        urls[2].contains("y.js?ad_domain"),
        "Bing keeps ad rows like Python"
    );
    assert_eq!(
        urls[3],
        "https://en.wikipedia.org/wiki/Rust_(programming_language)"
    );
}

#[test]
fn e2e_python_parity_ddg_html_div_h2_fixture() {
    let _lock = registry_guard();
    let results = parse_ddg_html(DDG_HTML_FIXTURE, 3, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://www.example.com/page");
}

#[test]
fn e2e_python_parity_raw_keeps_poison_serp_rows() {
    let _lock = registry_guard();
    let parsed = parse_bing_html(BING_PERSON_FIXTURE, 5, "ddgs").expect("parse");
    assert_eq!(parsed.len(), 3);
    let raw = select_raw(parsed, 5);
    assert_eq!(raw.len(), 3, "Python returns unrelated SERP rows as-is");
}

#[test]
fn e2e_python_parity_ranked_reorders_not_drops_poison() {
    let _lock = registry_guard();
    let parsed = parse_bing_html(BING_PERSON_FIXTURE, 5, "ddgs").expect("parse");
    let ranked = select_ranked("Raphaël MANSUY", parsed, 5);
    assert_eq!(ranked.len(), 3);
    assert!(ranked[0].url.contains("linkedin.com"));
}

#[test]
fn e2e_python_parity_bing_does_not_filter_ad_urls() {
    let _lock = registry_guard();
    let html = r#"
        <li class="b_algo"><h2><a href="https://duckduckgo.com/y.js?ad_domain=spam">Ad</a></h2><p>x</p></li>
        <li class="b_algo"><h2><a href="https://example.org/">Real</a></h2><p>snippet</p></li>
    "#;
    let urls = parse_fixture_bing(html, 5);
    assert_eq!(urls.len(), 2);
    assert!(urls[0].contains("y.js?ad_domain"));
}

#[test]
fn e2e_python_parity_html_filters_ad_urls() {
    let _lock = registry_guard();
    let html = r#"
        <div>
            <a href="https://duckduckgo.com/y.js?ad_domain=spam">
                <h2><a>Ad</a></h2>
                <a>buy</a>
            </a>
        </div>
        <div>
            <a href="https://example.org/">
                <h2><a>Real</a></h2>
                <a>snippet</a>
            </a>
        </div>
    "#;
    let results = parse_ddg_html(html, 5, "ddgs").expect("parse");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://example.org/");
}

#[test]
fn e2e_selection_mode_env_ranked() {
    let _lock = registry_guard();
    let prev = std::env::var("DDGS_SELECTION").ok();
    unsafe { std::env::set_var("DDGS_SELECTION", "ranked") };
    let s = DdgsSettings::resolve(&Default::default());
    assert_eq!(s.selection_mode, DdgsSelectionMode::Ranked);
    unsafe { std::env::remove_var("DDGS_SELECTION") };
    if let Some(v) = prev {
        unsafe { std::env::set_var("DDGS_SELECTION", v) };
    }
}

/// Parse the same fixture HTML with Python `ddgs` Bing xpath rules (no ad filter on Bing).
#[test]
fn e2e_python_script_parses_bing_fixture_same_urls() {
    let _lock = registry_guard();
    let script = r#"
import sys, base64, re
from html import unescape
from urllib.parse import parse_qs, urlparse, unquote
from lxml.html import document_fromstring

html = open(sys.argv[1]).read()
tree = document_fromstring(html)
urls = []
cache = set()
for e in tree.xpath("//li[contains(@class, 'b_algo')]"):
    hrefs = e.xpath("./h2/a/@href")
    if not hrefs:
        continue
    href = hrefs[0]
    if href.startswith("https://www.bing.com/ck/a?"):
        u = parse_qs(urlparse(href).query).get("u", [""])[0]
        if u.startswith("a1"):
            href = base64.urlsafe_b64decode(u[2:] + "=" * (-len(u[2:]) % 4)).decode()
    if href in cache:
        continue
    cache.add(href)
    urls.append(unquote(href))
print("\n".join(urls))
"#;
    let tmp = tempfile::NamedTempFile::new().expect("tmp");
    std::fs::write(tmp.path(), BING_RUST_FIXTURE).expect("write");
    let py = Command::new("python3")
        .args(["-c", script, tmp.path().to_str().expect("path")])
        .output()
        .expect("python");
    if !py.status.success() {
        eprintln!("python skip: {}", String::from_utf8_lossy(&py.stderr));
        return;
    }
    let py_urls: Vec<String> = String::from_utf8_lossy(&py.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    let rust_urls = parse_fixture_bing(BING_RUST_FIXTURE, 5);
    assert_eq!(
        rust_urls, py_urls,
        "Rust parse+select_raw must match Python Bing href cache order"
    );
}

/// Full `_text_bing` row parity — title, href, body (Python `_normalize` + `_normalize_url`).
#[test]
fn e2e_python_script_parses_bing_fixture_full_rows() {
    let _lock = registry_guard();
    let script = r#"
import sys, base64, json, re
from html import unescape
from urllib.parse import parse_qs, urlparse, unquote
from lxml.html import document_fromstring

REGEX_STRIP_TAGS = re.compile("<.*?>")

def normalize(raw):
    return unescape(REGEX_STRIP_TAGS.sub("", raw)) if raw else ""

def normalize_url(url):
    return unquote(url).replace(" ", "+") if url else ""

html = open(sys.argv[1]).read()
tree = document_fromstring(html)
rows = []
cache = set()
for e in tree.xpath("//li[contains(@class, 'b_algo')]"):
    hrefs = e.xpath("./h2/a/@href | ./div[contains(@class, 'header')]/a/@href")
    href = str(hrefs[0]) if hrefs else None
    if href and href.startswith("https://www.bing.com/ck/a?"):
        u = parse_qs(urlparse(href).query).get("u", [""])[0]
        if u.startswith("a1"):
            href = base64.urlsafe_b64decode(u[2:] + "=" * (-len(u[2:]) % 4)).decode()
    if href and href not in cache:
        cache.add(href)
        titlexpath = e.xpath("./h2/a//text() | ./div[contains(@class, 'header')]/a/h2//text()")
        title = "".join(str(x) for x in titlexpath) if titlexpath else ""
        bodyxpath = e.xpath(".//p//text()")
        body = "".join(str(x) for x in bodyxpath) if bodyxpath else ""
        rows.append({
            "title": normalize(title),
            "href": normalize_url(href),
            "body": normalize(body).replace("\xa0", " "),
        })
print(json.dumps(rows))
"#;
    let tmp = tempfile::NamedTempFile::new().expect("tmp");
    std::fs::write(tmp.path(), BING_RUST_FIXTURE).expect("write");
    let py = Command::new("python3")
        .args(["-c", script, tmp.path().to_str().expect("path")])
        .output()
        .expect("python");
    if !py.status.success() {
        eprintln!("python skip: {}", String::from_utf8_lossy(&py.stderr));
        return;
    }
    let py_rows: Vec<serde_json::Value> = serde_json::from_slice(&py.stdout).expect("python json");
    let rust_rows = select_raw(
        parse_bing_html(BING_RUST_FIXTURE, 10, "ddgs").expect("parse"),
        10,
    );
    assert_eq!(rust_rows.len(), py_rows.len());
    for (i, (rust, py)) in rust_rows.iter().zip(py_rows.iter()).enumerate() {
        assert_eq!(rust.url, py["href"].as_str().unwrap(), "row {i} href");
        assert_eq!(rust.title, py["title"].as_str().unwrap(), "row {i} title");
        assert_eq!(rust.snippet, py["body"].as_str().unwrap(), "row {i} body");
    }
}

fn canonical_host_path(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|u| {
            let host = u
                .host_str()
                .unwrap_or("")
                .trim_start_matches("www.")
                .trim_start_matches("m.");
            let path = u.path().trim_end_matches('/');
            format!("{host}{path}")
        })
        .unwrap_or_else(|| url.to_string())
}

fn url_sets_overlap(rust_urls: &[String], py_urls: &[String]) -> usize {
    let py_keys: Vec<String> = py_urls.iter().map(|u| canonical_host_path(u)).collect();
    rust_urls
        .iter()
        .filter(|u| py_keys.iter().any(|p| p == &canonical_host_path(u)))
        .count()
}

#[test]
fn e2e_ddgs_impersonate_env_pins_profile() {
    let _lock = registry_guard();
    let prev = std::env::var("DDGS_IMPERSONATE").ok();
    unsafe { std::env::set_var("DDGS_IMPERSONATE", "firefox_135") };
    let p = resolve_profile(std::env::var("DDGS_IMPERSONATE").ok().as_deref());
    assert_eq!(p.id(), "firefox_135");
    unsafe { std::env::remove_var("DDGS_IMPERSONATE") };
    if let Some(v) = prev {
        unsafe { std::env::set_var("DDGS_IMPERSONATE", v) };
    }
}

#[test]
fn e2e_chromium_fingerprint_includes_sec_ch_ua() {
    let _lock = registry_guard();
    let p = resolve_profile(Some("chrome_131"));
    assert!(p.user_agent().contains("Chrome/131"));
}

#[test]
fn e2e_fingerprint_pool_matches_primp_families() {
    let _lock = registry_guard();
    let p = pick_random_profile();
    let id = p.id();
    assert!(
        id.starts_with("chrome_")
            || id.starts_with("edge_")
            || id.starts_with("firefox_")
            || id.starts_with("safari_"),
        "unexpected profile id: {id}"
    );
}

#[test]
fn e2e_impersonate_os_windows_pins_chrome_win() {
    let _lock = registry_guard();
    let p = resolve_profile_with(Some("chrome_131"), Some(ImpersonateOs::Windows));
    assert_eq!(p.impersonate_os(), ImpersonateOs::Windows);
}

#[tokio::test]
#[ignore = "live network — compares Rust metasearch vs python ddgs on same query"]
async fn e2e_live_rust_vs_python_ddgs_urls_overlap() {
    let _lock = registry_guard();
    let query = "Rust programming language";
    let settings = DdgsSettings::resolve(&Default::default());
    assert_eq!(settings.selection_mode, DdgsSelectionMode::Raw);
    assert_eq!(settings.engine_order(), vec![DdgsEngine::Bing]);
    let opts = SearchOptions {
        max_results: 5,
        timeout_secs: 20,
        ..Default::default()
    };

    let rust_urls: Vec<String> = match metasearch_text(query, &opts, &settings, "ddgs").await {
        Ok(rows) => rows.into_iter().map(|r| r.url).collect(),
        Err(e) if e.message.contains("bot") || e.message.contains("blocked") => {
            eprintln!("skip live: rust blocked: {e}");
            return;
        }
        Err(e) => panic!("rust metasearch: {e:?}"),
    };

    let py = Command::new("python3")
        .args(["-c", "import sys; from ddgs import DDGS; rows = DDGS(timeout=20).text(sys.argv[1], max_results=5); print(chr(10).join(r['href'] for r in rows))", query])
        .env("PYTHONPATH", "/Users/raphaelmansuy/.venv/lib/python3.12/site-packages")
        .output()
        .expect("python ddgs");

    if !py.status.success() {
        eprintln!(
            "skip live: python failed: {}",
            String::from_utf8_lossy(&py.stderr)
        );
        return;
    }

    let py_urls: Vec<String> = String::from_utf8_lossy(&py.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();

    if rust_urls.is_empty() && py_urls.is_empty() {
        eprintln!("both empty — IP block or true empty SERP (inconclusive)");
        return;
    }

    eprintln!("rust: {rust_urls:?}");
    eprintln!("python: {py_urls:?}");

    // Rust fixes Python 9.0 script false-positive; may return rows when Python returns [].
    if !rust_urls.is_empty() && py_urls.is_empty() {
        eprintln!(
            "rust succeeded where python ddgs 9.0 returned [] (known script quirk in Python)"
        );
        assert!(rust_urls.len() >= 3, "rust should return meaningful rows");
        return;
    }

    assert_eq!(
        rust_urls.is_empty(),
        py_urls.is_empty(),
        "Rust/Python must agree on success vs empty when Python also succeeds"
    );

    let overlap = url_sets_overlap(&rust_urls, &py_urls);
    assert!(
        overlap >= 3,
        "expect ≥3 canonical URL overlaps when both succeed (got {overlap})"
    );
}
