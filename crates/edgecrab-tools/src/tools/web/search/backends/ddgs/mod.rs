//! DuckDuckGo search — native Rust reimplementation of the Python `ddgs` package.
//!
//! Layers (SOLID):
//! - `transport` — HTTP session + pacing
//! - `engines`   — Bing / DDG HTML / DDG lite parsers+pagination
//! - `text`      — HTML decode + snippet hygiene (DRY)
//! - `relevance` — query tokens + spam batch detection
//! - `rank`      — merge engines, score, select best hits
//! - `metasearch` — orchestration across engines + variants
//! - `parse` / `detect` / `error` — shared utilities

mod detect;
mod engines;
mod error;
mod metasearch;
mod parse;
mod rank;
mod relevance;
mod settings;
mod text;
mod transport;

pub use parse::{normalize_bing_url, normalize_ddg_url, parse_bing_html, parse_ddg_html, parse_ddg_lite, parse_engine_html, engine_reports_no_results};
pub use detect::{is_bot_challenge, is_engine_blocked};
pub use relevance::filter_relevant;
pub use settings::DdgsEngine;

use async_trait::async_trait;

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::config::SearchOptions;
use crate::tools::web::search::error::SearchError;

use self::settings::DdgsSettings;

pub struct DdgsBackend;

#[async_trait]
impl WebSearchBackend for DdgsBackend {
    fn name(&self) -> &str {
        "ddgs"
    }

    fn display_name(&self) -> &str {
        "DuckDuckGo (ddgs)"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let settings = DdgsSettings::resolve(&opts.backend_config);
        metasearch::search_text(query, opts, &settings, self.name()).await
    }
}

#[cfg(test)]
mod tests {
    use super::detect;
    use super::parse;

    #[test]
    fn module_parse_reexported() {
        let html = r#"<a class="result__a" href="https://example.com">Ex</a>"#;
        assert_eq!(parse::parse_ddg_html(html, 3, "ddgs").expect("ok").len(), 1);
    }

    #[test]
    fn detect_exported_for_chain_tests() {
        assert!(detect::is_bot_challenge("anomaly-modal"));
    }
}

#[cfg(test)]
mod live_engine_tests {
    use super::settings::DdgsSettings;
    use super::transport::DdgsSession;

    #[tokio::test]
    #[ignore = "live network diagnostic"]
    async fn bing_wreq_diagnostic() {
        let settings = DdgsSettings::default();
        let mut session = DdgsSession::new(15).expect("session");
        session.warm_up("ddgs").await.expect("warmup");
        let region = settings.region.as_str();
        let cookie = format!("_EDGE_CD=u={region}&m={region}; _EDGE_S=ui={region}&mkt={region}");
        let html = session
            .get(
                super::settings::DdgsEngine::Bing,
                "ddgs",
                "https://www.bing.com/search",
                &[("q", "Rust programming language")],
                Some(&cookie),
            )
            .await
            .expect("bing get");
        eprintln!(
            "bing html len={} b_algo={} captcha={}",
            html.len(),
            html.matches("b_algo").count(),
            html.to_ascii_lowercase().contains("captcha")
        );
        let parsed = super::parse::parse_bing_html(&html, 5, "ddgs").expect("parse");
        eprintln!("parsed results={}", parsed.len());
        if let Some(first) = parsed.first() {
            eprintln!("first url={}", first.url);
            assert!(
                !first.url.contains("bing.com/ck/a"),
                "bing ck redirect should be decoded: {}",
                first.url
            );
        }
    }
}

#[cfg(test)]
mod live_tests {
    use super::metasearch;
    use super::settings::DdgsSettings;
    use crate::config_ref::WebSearchBackendConfigRef;
    use crate::tools::web::search::config::SearchOptions;

    #[tokio::test]
    #[ignore = "live network — run with --ignored"]
    async fn metasearch_finds_public_results() {
        let settings = DdgsSettings::resolve(&WebSearchBackendConfigRef::default());
        let opts = SearchOptions {
            max_results: 3,
            timeout_secs: 15,
            ..Default::default()
        };
        let results = metasearch::search_text("Rust programming language", &opts, &settings, "ddgs")
            .await
            .unwrap_or_else(|e| {
                if e.message.contains("bot challenge") || e.message.contains("blocked") {
                    eprintln!("Skipping live ddgs: {e}");
                    return Vec::new();
                }
                panic!("metasearch should succeed: {e:?}");
            });
        if !results.is_empty() {
            assert!(results[0].url.starts_with("http"));
        }
    }
}
