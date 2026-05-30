//! DuckDuckGo / Bing HTML → normalized [`SearchResult`] rows (Python `ddgs` parsers).

use std::collections::HashMap;
use std::sync::OnceLock;

use base64::Engine as _;
use regex::Regex;

use super::settings::DdgsEngine;
use super::text;
use crate::tools::web::search::backend::SearchResult;
use crate::tools::web::search::error::SearchError;

static DDG_DIV_H2_RE: OnceLock<Regex> = OnceLock::new();
static DDG_RESULT_RE: OnceLock<Regex> = OnceLock::new();
static DDG_SNIPPET_RE: OnceLock<Regex> = OnceLock::new();
static DDG_ALT_LINK_RE: OnceLock<Regex> = OnceLock::new();
static DDG_NAV_LINK_RE: OnceLock<Regex> = OnceLock::new();
static DDG_HIDDEN_INPUT_RE: OnceLock<Regex> = OnceLock::new();
static DDG_LITE_FORM_RE: OnceLock<Regex> = OnceLock::new();
static DDG_LITE_TR_RE: OnceLock<Regex> = OnceLock::new();
static BING_ALGO_RE: OnceLock<Regex> = OnceLock::new();
static BING_ALGO_ALT_RE: OnceLock<Regex> = OnceLock::new();

/// Route HTML to the correct parser for the metasearch engine.
pub fn parse_engine_html(
    engine: DdgsEngine,
    html: &str,
    max: usize,
    source: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    match engine {
        DdgsEngine::Html => parse_ddg_html(html, max, source),
        DdgsEngine::Lite => parse_ddg_lite(html, max, source),
        DdgsEngine::Bing => parse_bing_html(html, max, source),
    }
}

/// True when the engine HTML explicitly states zero hits (distinct from parse failure).
pub fn engine_reports_no_results(engine: DdgsEngine, html: &str) -> bool {
    match engine {
        DdgsEngine::Html => html.contains("No  results."),
        DdgsEngine::Lite => html.contains("No more results."),
        // Bing embeds `"There are no results for…"` in client-side JSON even when `b_algo`
        // organic blocks are present — never short-circuit on that string alone.
        DdgsEngine::Bing => html.contains("b_noResults") && !html.contains("b_algo"),
    }
}

/// Decode Bing redirect URLs (`/ck/a?u=a...` base64) to the target href.
pub fn normalize_bing_url(raw: &str) -> Option<String> {
    let raw = text::decode_html_entities(raw.trim());
    if raw.is_empty() {
        return None;
    }

    if raw.starts_with("https://www.bing.com/ck/a?") || raw.starts_with("http://www.bing.com/ck/a?") {
        if let Some(decoded) = decode_bing_ck_target(&raw) {
            return Some(decoded);
        }
    }

    Some(raw.replace('\u{00a0}', " "))
}

fn decode_bing_ck_target(raw: &str) -> Option<String> {
    if let Ok(parsed) = url::Url::parse(raw) {
        for (key, value) in parsed.query_pairs() {
            if key == "u" {
                if let Some(decoded) = decode_bing_u_param(value.as_ref()) {
                    return Some(decoded);
                }
            }
        }
    }

    // Fallback when `url::Url` rejects Bing's `?!&&` query shape.
    static U_PARAM: OnceLock<Regex> = OnceLock::new();
    let re = U_PARAM.get_or_init(|| Regex::new(r"[?&]u=([^&]+)").expect("bing u param regex"));
    re.captures(raw)
        .and_then(|cap| cap.get(1))
        .and_then(|m| decode_bing_u_param(m.as_str()))
}

fn decode_bing_u_param(encoded: &str) -> Option<String> {
    let payload = encoded.get(2..).unwrap_or(encoded);
    let pad = (4 - payload.len() % 4) % 4;
    let padded = format!("{payload}{}", "=".repeat(pad));
    let decoded = base64::engine::general_purpose::URL_SAFE
        .decode(padded.as_bytes())
        .ok()?;
    String::from_utf8(decoded)
        .ok()
        .map(|url| url.replace('\u{00a0}', " "))
}

/// Decode DDG redirect URLs (`/l/?uddg=…`) to the target href (Hermes uses package-normalized URLs).
pub fn normalize_ddg_url(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    if (raw.contains("duckduckgo.com/l/") || raw.contains("duckduckgo.com/l/?"))
        && let Some(target) = extract_uddg_param(raw)
    {
        return decode_percent_encoding(target);
    }

    if raw.starts_with("//") {
        return Some(format!("https:{}", raw));
    }

    Some(raw.to_string())
}

fn extract_uddg_param(url: &str) -> Option<&str> {
    let lower = url.to_ascii_lowercase();
    let idx = lower.find("uddg=")?;
    let rest = &url[idx + 5..];
    let end = rest.find('&').unwrap_or(rest.len());
    Some(&rest[..end])
}

fn decode_percent_encoding(input: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let b = input.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else if b[i] == b'+' {
            bytes.push(b' ');
            i += 1;
        } else {
            bytes.push(b[i]);
            i += 1;
        }
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Extract next-page form payload from DDG HTML (`nav-link` hidden inputs).
pub fn extract_ddg_html_next_payload(html: &str) -> Option<HashMap<String, String>> {
    let nav_re = DDG_NAV_LINK_RE.get_or_init(|| {
        Regex::new(r#"(?s)<div[^>]*class="nav-link"[^>]*>(.*?)</div>"#).expect("nav-link regex")
    });
    let hidden_re = DDG_HIDDEN_INPUT_RE.get_or_init(|| {
        Regex::new(r#"<input[^>]*type="hidden"[^>]*name="([^"]+)"[^>]*value="([^"]*)""#)
            .expect("hidden input regex")
    });
    let nav = nav_re.captures_iter(html).last()?.get(1)?.as_str();
    let mut payload = HashMap::new();
    for cap in hidden_re.captures_iter(nav) {
        payload.insert(cap[1].to_string(), cap[2].to_string());
    }
    if payload.is_empty() { None } else { Some(payload) }
}

/// Extract next-page form payload from DDG lite (`form` with `ext` submit).
pub fn extract_ddg_lite_next_payload(html: &str) -> Option<HashMap<String, String>> {
    let form_re = DDG_LITE_FORM_RE.get_or_init(|| {
        Regex::new(r#"(?s)<form[^>]*>.*?value="[^"]*ext[^"]*".*?</form>"#).expect("lite form regex")
    });
    let hidden_re = DDG_HIDDEN_INPUT_RE.get_or_init(|| {
        Regex::new(r#"<input[^>]*type="hidden"[^>]*name="([^"]+)"[^>]*value="([^"]*)""#)
            .expect("hidden input regex")
    });
    let form = form_re.captures_iter(html).last()?.get(0)?.as_str();
    let mut payload = HashMap::new();
    for cap in hidden_re.captures_iter(form) {
        payload.insert(cap[1].to_string(), cap[2].to_string());
    }
    if payload.is_empty() { None } else { Some(payload) }
}

fn is_ddg_ad_url(url: &str) -> bool {
    url.starts_with("http://www.google.com/search?q=")
        || url.contains("duckduckgo.com/y.js?ad_domain")
}

/// Parse DDG HTML — Python xpath `//div[h2]` primary, `result__a` fallback.
pub fn parse_ddg_html(
    html: &str,
    max: usize,
    source: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    if let Some(results) = parse_ddg_html_div_h2(html, max, source) {
        if !results.is_empty() {
            return Ok(results);
        }
    }

    let result_re = DDG_RESULT_RE.get_or_init(|| {
        Regex::new(r#"class="result__a"[^>]*href="([^"]+)"[^>]*>([^<]+)"#)
            .expect("valid DDG result regex")
    });
    let snippet_re = DDG_SNIPPET_RE.get_or_init(|| {
        Regex::new(r#"class="result__snippet"[^>]*>([\s\S]*?)</a>"#)
            .expect("valid DDG snippet regex")
    });
    let alt_re = DDG_ALT_LINK_RE.get_or_init(|| {
        Regex::new(r#"class="result__url"[^>]*href="([^"]+)"#).expect("valid DDG alt url regex")
    });

    let mut pairs: Vec<(String, String)> = result_re
        .captures_iter(html)
        .filter_map(|c| {
            let url = normalize_ddg_url(&c[1])?;
            let title = c[2].trim().to_string();
            if title.is_empty() {
                return None;
            }
            Some((url, title))
        })
        .collect();

    if pairs.is_empty() {
        pairs = alt_re
            .captures_iter(html)
            .filter_map(|c| {
                let url = normalize_ddg_url(&c[1])?;
                Some((url, String::new()))
            })
            .collect();
    }

    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .map(|c| text::clean_snippet(&c[1]))
        .collect();

    finish_ddg_pairs(pairs, snippets, max, source)
}

/// Python `_text_duckduckgo_html`: `//div[h2]` blocks.
fn parse_ddg_html_div_h2(html: &str, max: usize, source: &str) -> Option<Vec<SearchResult>> {
    let div_h2_re = DDG_DIV_H2_RE.get_or_init(|| {
        Regex::new(
            r#"(?s)<div[^>]*>\s*<a[^>]*href="([^"]+)"[^>]*>.*?<h2>\s*<a[^>]*>([^<]*)</a>\s*</h2>.*?<a[^>]*>([\s\S]*?)</a>"#,
        )
        .expect("valid DDG div[h2] regex")
    });

    let mut out = Vec::new();
    for cap in div_h2_re.captures_iter(html).take(max) {
        let url = normalize_ddg_url(&cap[1])?;
        if is_ddg_ad_url(&url) {
            continue;
        }
        let title = text::clean_title(cap[2].trim());
        let snippet = text::clean_snippet(cap.get(3).map(|m| m.as_str()).unwrap_or(""));
        let title = if title.is_empty() { url.clone() } else { title };
        out.push(SearchResult::new(
            out.len() + 1,
            title,
            url,
            snippet,
            source,
        ));
    }
    Some(out)
}

fn finish_ddg_pairs(
    pairs: Vec<(String, String)>,
    snippets: Vec<String>,
    max: usize,
    source: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    let mut out = Vec::new();
    for (i, (url, title)) in pairs.into_iter().take(max).enumerate() {
        let snippet = snippets.get(i).cloned().unwrap_or_default();
        let title = if title.is_empty() {
            url.clone()
        } else {
            text::clean_title(&title)
        };
        out.push(SearchResult::new(i + 1, title, url, snippet, source));
    }
    Ok(out)
}

/// Parse DDG lite — Python `//table[last()]//tr` row groups (link row + snippet row).
pub fn parse_ddg_lite(
    html: &str,
    max: usize,
    source: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    if html.contains("No more results.") {
        return Ok(Vec::new());
    }

    let tr_re = DDG_LITE_TR_RE.get_or_init(|| {
        Regex::new(r"(?s)<tr[^>]*>(.*?)</tr>").expect("lite tr regex")
    });
    let href_re = Regex::new(r#"<a[^>]*href="([^"]+)""#).expect("href regex");
    let title_re = Regex::new(r#"<a[^>]*>([^<]*)</a>"#).expect("title regex");
    let snippet_re = Regex::new(r#"class='result-snippet'[^>]*>([\s\S]*?)</td>"#).expect("snippet");

    let rows: Vec<&str> = tr_re
        .captures_iter(html)
        .filter_map(|c| c.get(1).map(|m| m.as_str()))
        .collect();

    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < rows.len() && out.len() < max {
        let link_row = rows[i];
        let snippet_row = rows[i + 1];

        if let Some(href_cap) = href_re.captures(link_row) {
            if let Some(url) = normalize_ddg_url(&href_cap[1]) {
                if !is_ddg_ad_url(&url) {
                    let title = title_re
                        .captures(link_row)
                        .map(|c| text::clean_title(c[1].trim()))
                        .unwrap_or_default();
                    let snippet = snippet_re
                        .captures(snippet_row)
                        .map(|c| text::clean_snippet(&c[1]))
                        .unwrap_or_default();
                    let title = if title.is_empty() { url.clone() } else { title };
                    out.push(SearchResult::new(out.len() + 1, title, url, snippet, source));
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }

    Ok(out)
}

/// Parse Bing HTML search results (`www.bing.com/search`).
pub fn parse_bing_html(
    html: &str,
    max: usize,
    source: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    let algo_re = BING_ALGO_RE.get_or_init(|| {
        Regex::new(
            r#"(?s)<li[^>]*class="[^"]*b_algo[^"]*"[^>]*>.*?<h2[^>]*>\s*<a[^>]*href="([^"]+)"[^>]*>([\s\S]*?)</a>\s*</h2>.*?<p[^>]*>([\s\S]*?)</p>"#,
        )
        .expect("valid Bing b_algo regex")
    });
    let algo_alt_re = BING_ALGO_ALT_RE.get_or_init(|| {
        Regex::new(
            r#"(?s)<li[^>]*class="[^"]*b_algo[^"]*"[^>]*>.*?<div[^>]*class="[^"]*header[^"]*"[^>]*>\s*<a[^>]*href="([^"]+)"[^>]*>.*?<h2[^>]*>([\s\S]*?)</h2>"#,
        )
        .expect("valid Bing b_algo alt regex")
    });

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in algo_re.captures_iter(html) {
        if out.len() >= max {
            break;
        }
        push_bing_result(&mut out, &mut seen, &cap, source, true);
    }
    if out.len() < max {
        for cap in algo_alt_re.captures_iter(html) {
            if out.len() >= max {
                break;
            }
            push_bing_result(&mut out, &mut seen, &cap, source, false);
        }
    }
    Ok(out)
}

fn push_bing_result(
    out: &mut Vec<SearchResult>,
    seen: &mut std::collections::HashSet<String>,
    cap: &regex::Captures,
    source: &str,
    with_snippet: bool,
) {
    let raw_url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
    let Some(url) = normalize_bing_url(raw_url) else {
        return;
    };
    if url.is_empty() || !seen.insert(url.clone()) {
        return;
    }
    let title = cap
        .get(2)
        .map(|m| text::clean_title(m.as_str()))
        .unwrap_or_default();
    let snippet = if with_snippet {
        cap.get(3)
            .map(|m| text::clean_snippet(m.as_str()))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let title = if title.is_empty() { url.clone() } else { title };
    out.push(SearchResult::new(out.len() + 1, title, url, snippet, source));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::web::search::backends::ddgs::settings::DdgsEngine;

    #[test]
    fn parse_classic_result_and_snippet() {
        let html = r#"
            <a class="result__a" href="https://rust-lang.org">Rust</a>
            <a class="result__snippet">Systems programming language</a>
        "#;
        let results = parse_ddg_html(html, 5, "ddgs").expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust");
        assert_eq!(results[0].url, "https://rust-lang.org");
        assert!(results[0].snippet.contains("Systems"));
    }

    #[test]
    fn parse_empty_html_is_success() {
        assert!(
            parse_ddg_html("<html></html>", 5, "ddgs")
                .expect("parse")
                .is_empty()
        );
    }

    #[test]
    fn decode_ddg_redirect_url() {
        let raw = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage";
        let url = normalize_ddg_url(raw).expect("decode");
        assert_eq!(url, "https://example.com/page");
    }

    #[test]
    fn protocol_relative_urls_normalized() {
        assert_eq!(
            normalize_ddg_url("//example.com/x").as_deref(),
            Some("https://example.com/x")
        );
    }

    #[test]
    fn href_or_url_style_passthrough() {
        assert_eq!(
            normalize_ddg_url("https://direct.example").as_deref(),
            Some("https://direct.example")
        );
    }

    #[test]
    fn parse_bing_ignores_embedded_js_no_results_string() {
        let html = r#"
            <script>"There are no results for this question, please check your spelling"</script>
            <li class="b_algo">
                <h2><a href="https://rust-lang.org">Rust Programming Language</a></h2>
                <p>A systems programming language.</p>
            </li>
        "#;
        let results = parse_bing_html(html, 5, "ddgs").expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://rust-lang.org");
        assert!(!engine_reports_no_results(DdgsEngine::Bing, html));
    }

    #[test]
    fn engine_reports_no_results_bing_requires_b_no_results() {
        assert!(!engine_reports_no_results(
            DdgsEngine::Bing,
            r#"<script>"There are no results for"</script><li class="b_algo"></li>"#
        ));
        assert!(engine_reports_no_results(
            DdgsEngine::Bing,
            r#"<div class="b_noResults">No results</div>"#
        ));
    }

    #[test]
    fn parse_bing_algo_with_h2_attributes() {
        let html = r#"
            <li class="b_algo" data-id="">
                <h2 class=""><a href="https://www.rust-lang.org/"><strong>Rust Programming Language</strong></a></h2>
                <div class="b_caption"><p class="b_lineclamp2">Blazingly fast systems language.</p></div>
            </li>
        "#;
        let results = parse_bing_html(html, 5, "ddgs").expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming Language");
        assert!(results[0].snippet.contains("Blazingly"));
    }

    #[test]
    fn parse_bing_algo_block() {
        let html = r#"
            <li class="b_algo">
                <h2><a href="https://example.com/page">Example Site</a></h2>
                <p>A useful snippet about the page.</p>
            </li>
        "#;
        let results = parse_bing_html(html, 5, "ddgs").expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example Site");
        assert_eq!(results[0].url, "https://example.com/page");
        assert!(results[0].snippet.contains("useful snippet"));
    }

    #[test]
    fn parse_bing_algo_html_with_ck_amp_href() {
        let html = r#"
            <li class="b_algo" data-id="">
                <h2 class=""><a href="https://www.bing.com/ck/a?!&amp;&amp;p=abc&amp;u=a1aHR0cHM6Ly9leGFtcGxlLmNvbS8&amp;ntb=1"><strong>Example</strong></a></h2>
                <div class="b_caption"><p class="b_lineclamp2">Snippet text.</p></div>
            </li>
        "#;
        let results = parse_bing_html(html, 5, "ddgs").expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com/");
    }

    #[test]
    fn parse_bing_live_ck_redirect_with_amp_entities() {
        let raw = "https://www.bing.com/ck/a?!&amp;&amp;p=abc&amp;u=a1aHR0cHM6Ly9ydXN0LWxhbmcub3JnLw&amp;ntb=1";
        let url = normalize_bing_url(raw).expect("decode live bing ck url");
        assert_eq!(url, "https://rust-lang.org/");
    }

    #[test]
    fn parse_bing_live_ck_redirect_modern_query_shape() {
        let raw = "https://www.bing.com/ck/a?!&&p=abc&u=a1aHR0cHM6Ly9ydXN0LWxhbmcub3JnLw&ntb=1";
        let url = normalize_bing_url(raw).expect("decode modern bing ck url");
        assert_eq!(url, "https://rust-lang.org/");
    }

    #[test]
    fn parse_bing_ck_redirect() {
        let inner = "https://decoded.example/path";
        let encoded = base64::engine::general_purpose::URL_SAFE.encode(inner.as_bytes());
        let u_param = format!("xx{encoded}");
        let html = format!(
            r#"<li class="b_algo"><h2><a href="https://www.bing.com/ck/a?u={u_param}">T</a></h2><p>body</p></li>"#
        );
        let results = parse_bing_html(&html, 5, "ddgs").expect("parse");
        assert_eq!(results[0].url, inner);
    }

    #[test]
    fn parse_engine_routes_to_bing() {
        let html = r#"<li class="b_algo"><h2><a href="https://x.com">X</a></h2><p>s</p></li>"#;
        let results =
            parse_engine_html(DdgsEngine::Bing, html, 3, "ddgs").expect("parse");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn parse_ddg_lite_table_rows() {
        let html = r#"
            <table><tr><td><a href="https://example.org">Example Org</a></td></tr>
            <tr><td class='result-snippet'>Snippet for example org</td></tr></table>
        "#;
        let results = parse_ddg_lite(html, 5, "ddgs").expect("parse");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.org");
    }
}
