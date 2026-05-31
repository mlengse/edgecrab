//! Metasearch engines — Bing, DDG HTML, DDG lite (Open/Closed: add engines here).

use std::collections::HashMap;

use super::parse;
use super::settings::{DdgsEngine, DdgsSettings};
use super::transport::DdgsSession;
use crate::tools::web::search::backend::SearchResult;
use crate::tools::web::search::error::SearchError;

const DDG_HTML_URL: &str = "https://html.duckduckgo.com/html";
const DDG_LITE_URL: &str = "https://lite.duckduckgo.com/lite/";
const BING_SEARCH_URL: &str = "https://www.bing.com/search";
const MAX_PAGES: usize = 5;

struct FormEngineQuery<'a> {
    engine: DdgsEngine,
    url: &'a str,
    referer: &'a str,
    query: &'a str,
    settings: &'a DdgsSettings,
    max: usize,
    backend: &'a str,
}

/// Run one metasearch engine to completion (pagination included).
pub async fn run_engine(
    session: &mut DdgsSession,
    engine: DdgsEngine,
    query: &str,
    settings: &DdgsSettings,
    max: usize,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    match engine {
        DdgsEngine::Bing => search_bing(session, query, settings, max, backend).await,
        DdgsEngine::Html => search_ddg_html(session, query, settings, max, backend).await,
        DdgsEngine::Lite => search_ddg_lite(session, query, settings, max, backend).await,
    }
}

fn bing_region_cookie(settings: &DdgsSettings) -> Option<String> {
    settings.region().map(|region| {
        format!("_EDGE_CD=u={region}&m={region}; _EDGE_S=ui={region}&mkt={region}")
    })
}

async fn search_bing(
    session: &mut DdgsSession,
    query: &str,
    settings: &DdgsSettings,
    max: usize,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    let cookie = bing_region_cookie(settings);
    let mut results = Vec::new();
    let mut payload: HashMap<&str, String> = HashMap::from([("q", query.to_string())]);

    for page in 0..MAX_PAGES {
        let params: Vec<(&str, &str)> = payload.iter().map(|(k, v)| (*k, v.as_str())).collect();

        let html = session
            .get(
                DdgsEngine::Bing,
                backend,
                BING_SEARCH_URL,
                &params,
                cookie.as_deref(),
            )
            .await?;

        if parse::bing_page_reports_no_results(&html) {
            break;
        }

        if let Some(batch) = process_page(
            DdgsEngine::Bing,
            &html,
            max.saturating_sub(results.len()),
            backend,
        )? {
            let n = batch.len();
            results.extend(batch);
            if results.len() >= max || n == 0 {
                break;
            }
        } else {
            break;
        }

        if results.len() >= max {
            break;
        }

        if max == 0 {
            break;
        }

        // Python `_text_bing`: replace `first`/`FORM` keys each page (not accumulate).
        let first = ((page + 1) * 10 + 1).to_string();
        let form = if page > 0 {
            format!("PERE{page}")
        } else {
            "PERE".into()
        };
        payload.insert("first", first);
        payload.insert("FORM", form);
    }
    Ok(truncate(results, max))
}

async fn search_ddg_form_engine(
    session: &mut DdgsSession,
    req: FormEngineQuery<'_>,
) -> Result<Vec<SearchResult>, SearchError> {
    let FormEngineQuery {
        engine,
        url,
        referer,
        query,
        settings,
        max,
        backend,
    } = req;
    let mut payload: HashMap<String, String> = HashMap::from([
        ("q".into(), query.to_string()),
        ("b".into(), String::new()),
    ]);
    if let Some(region) = settings.region() {
        payload.insert("kl".into(), region.to_string());
    }
    let mut results = Vec::new();

    for _ in 0..MAX_PAGES {
        let form: Vec<(&str, &str)> = payload
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let html = session
            .post_form(engine, backend, url, referer, &form)
            .await?;

        if let Some(batch) =
            process_page(engine, &html, max.saturating_sub(results.len()), backend)?
        {
            let n = batch.len();
            results.extend(batch);
            if results.len() >= max || n == 0 {
                break;
            }
            let next = match engine {
                DdgsEngine::Html => parse::extract_ddg_html_next_payload(&html),
                DdgsEngine::Lite => parse::extract_ddg_lite_next_payload(&html),
                DdgsEngine::Bing => None,
            };
            if let Some(p) = next {
                payload = p;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    Ok(truncate(results, max))
}

async fn search_ddg_html(
    session: &mut DdgsSession,
    query: &str,
    settings: &DdgsSettings,
    max: usize,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    search_ddg_form_engine(
        session,
        FormEngineQuery {
            engine: DdgsEngine::Html,
            url: DDG_HTML_URL,
            referer: "https://html.duckduckgo.com/",
            query,
            settings,
            max,
            backend,
        },
    )
    .await
}

async fn search_ddg_lite(
    session: &mut DdgsSession,
    query: &str,
    settings: &DdgsSettings,
    max: usize,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    search_ddg_form_engine(
        session,
        FormEngineQuery {
            engine: DdgsEngine::Lite,
            url: DDG_LITE_URL,
            referer: "https://lite.duckduckgo.com/",
            query,
            settings,
            max,
            backend,
        },
    )
    .await
}

/// Parse one page; `None` = stop pagination. Python parity: HTTP status gates errors, not HTML heuristics.
fn process_page(
    engine: DdgsEngine,
    html: &str,
    max: usize,
    backend: &str,
) -> Result<Option<Vec<SearchResult>>, SearchError> {
    if parse::engine_reports_no_results(engine, html) {
        return Ok(Some(Vec::new()));
    }
    let batch = parse::parse_engine_html(engine, html, max, backend)?;
    Ok(Some(batch))
}

fn truncate(mut results: Vec<SearchResult>, max: usize) -> Vec<SearchResult> {
    if max > 0 {
        results.truncate(max);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bing_cookie_only_when_region_set() {
        assert!(bing_region_cookie(&DdgsSettings::default()).is_none());
        let mut s = DdgsSettings::default();
        s.region = "fr-fr".into();
        assert!(bing_region_cookie(&s).unwrap().contains("fr-fr"));
    }

    #[test]
    fn bing_pagination_replaces_first_and_form_keys() {
        let mut payload: HashMap<&str, String> = HashMap::from([("q", "rust".into())]);
        for page in 0..3 {
            let first = ((page + 1) * 10 + 1).to_string();
            let form = if page > 0 {
                format!("PERE{page}")
            } else {
                "PERE".into()
            };
            payload.insert("first", first.clone());
            payload.insert("FORM", form.clone());
            assert_eq!(payload.len(), 3);
            assert_eq!(payload.get("first").unwrap(), &first);
            assert_eq!(payload.get("FORM").unwrap(), &form);
        }
    }

    #[test]
    fn captcha_html_without_serp_returns_empty_like_python() {
        let html = r#"<html><body>captcha challenge verify you are human bing.com</body></html>"#;
        let batch = process_page(DdgsEngine::Bing, html, 5, "ddgs")
            .expect("parse")
            .expect("some");
        assert!(batch.is_empty(), "Python returns [] on HTTP 200 + no b_algo");
    }
}
