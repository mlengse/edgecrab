//! Metasearch engines — Bing, DDG HTML, DDG lite (Open/Closed: add engines here).

use std::collections::HashMap;

use super::detect;
use super::parse;
use super::settings::{DdgsEngine, DdgsSettings};
use super::transport::DdgsSession;
use crate::tools::web::search::backend::SearchResult;
use crate::tools::web::search::error::SearchError;

const DDG_HTML_URL: &str = "https://html.duckduckgo.com/html";
const DDG_LITE_URL: &str = "https://lite.duckduckgo.com/lite/";
const BING_SEARCH_URL: &str = "https://www.bing.com/search";
const MAX_PAGES: usize = 5;

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

async fn search_bing(
    session: &mut DdgsSession,
    query: &str,
    settings: &DdgsSettings,
    max: usize,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    let region = settings.region.as_str();
    let cookie = format!("_EDGE_CD=u={region}&m={region}; _EDGE_S=ui={region}&mkt={region}");
    let mut results = Vec::new();
    let mut page_params: Vec<(&str, String)> = vec![("q", query.to_string())];

    for page in 0..MAX_PAGES {
        let params: Vec<(&str, &str)> = page_params
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        let html = session
            .get(DdgsEngine::Bing, backend, BING_SEARCH_URL, &params, Some(&cookie))
            .await?;

        if let Some(batch) = process_page(DdgsEngine::Bing, &html, max.saturating_sub(results.len()), backend)? {
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

        // Python ddgs: pagination params added after the first response.
        let first = ((page + 1) * 10 + 1).to_string();
        let form = if page > 0 {
            format!("PERE{page}")
        } else {
            "PERE".into()
        };
        page_params.push(("first", first));
        page_params.push(("FORM", form));
    }
    Ok(truncate(results, max))
}

async fn search_ddg_form_engine(
    session: &mut DdgsSession,
    engine: DdgsEngine,
    url: &str,
    referer: &str,
    query: &str,
    settings: &DdgsSettings,
    max: usize,
    backend: &str,
) -> Result<Vec<SearchResult>, SearchError> {
    let mut payload: HashMap<String, String> = HashMap::from([
        ("q".into(), query.to_string()),
        ("b".into(), String::new()),
        ("kl".into(), settings.region.clone()),
    ]);
    let mut results = Vec::new();

    for _ in 0..MAX_PAGES {
        let form: Vec<(&str, &str)> = payload
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let html = session
            .post_form(engine, backend, url, referer, &form)
            .await?;

        if let Some(batch) = process_page(engine, &html, max.saturating_sub(results.len()), backend)? {
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
        DdgsEngine::Html,
        DDG_HTML_URL,
        "https://html.duckduckgo.com/",
        query,
        settings,
        max,
        backend,
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
        DdgsEngine::Lite,
        DDG_LITE_URL,
        "https://lite.duckduckgo.com/",
        query,
        settings,
        max,
        backend,
    )
    .await
}

/// Parse one page; `None` = stop pagination (blocked, zero-hit page, or explicit empty).
fn process_page(
    engine: DdgsEngine,
    html: &str,
    max: usize,
    backend: &str,
) -> Result<Option<Vec<SearchResult>>, SearchError> {
    if detect::is_engine_blocked(html) {
        return Err(SearchError::server(
            backend,
            503,
            format!("{} blocked this request (bot challenge).", engine.label()),
        ));
    }
    if parse::engine_reports_no_results(engine, html) {
        return Ok(Some(Vec::new()));
    }
    let batch = parse::parse_engine_html(engine, html, max, backend)?;
    Ok(Some(batch))
}

fn truncate(mut results: Vec<SearchResult>, max: usize) -> Vec<SearchResult> {
    results.truncate(max);
    results
}
