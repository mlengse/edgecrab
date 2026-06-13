//! SearXNG meta-search backend (self-hosted, zero marginal API cost).

use async_trait::async_trait;
use serde_json::Value;

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::backend_settings::require_endpoint;
use crate::tools::web::search::config::SearchOptions;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error, validate_search_url};

pub struct SearxngBackend;

#[async_trait]
impl WebSearchBackend for SearxngBackend {
    fn name(&self) -> &str {
        "searxng"
    }

    fn is_available(&self) -> bool {
        std::env::var("SEARXNG_URL")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let base = require_endpoint(self.name(), &opts.backend_config, &["SEARXNG_URL"])?;
        let url = format!("{base}/search");
        validate_search_url(&url)?;

        let client = build_api_client(opts.timeout_secs)?;
        let resp = client
            .get(&url)
            .query(&[("q", query), ("format", "json"), ("pageno", "1")])
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| map_reqwest_error(self.name(), e))?;

        let code = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SearchError::from_http_status(
                self.name(),
                code,
                format!("SearXNG HTTP {code}: {text}"),
            ));
        }

        let data: Value = resp.json().await.map_err(|e| {
            SearchError::hard(self.name(), format!("SearXNG JSON parse error: {e}"))
        })?;

        Ok(parse_searxng_json(&data, opts.max_results(), self.name()))
    }
}

/// Normalize SearXNG JSON into ranked [`SearchResult`] rows (score-desc, 1-indexed rank).
pub(crate) fn parse_searxng_json(data: &Value, max: usize, source: &str) -> Vec<SearchResult> {
    let mut raw: Vec<(f64, Value)> = data
        .get("results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            let score = r.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
            (score, r)
        })
        .collect();
    raw.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    raw.into_iter()
        .take(max)
        .enumerate()
        .filter_map(|(i, (_, r))| {
            let title = r.get("title")?.as_str()?.to_string();
            let url = r.get("url")?.as_str()?.to_string();
            let snippet = r
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(SearchResult::new(i + 1, title, url, snippet, source))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_without_url() {
        let prev = std::env::var("SEARXNG_URL").ok();
        unsafe { std::env::remove_var("SEARXNG_URL") };
        assert!(!SearxngBackend.is_available());
        if let Some(v) = prev {
            unsafe { std::env::set_var("SEARXNG_URL", v) };
        }
    }

    #[tokio::test]
    async fn ssrf_blocks_private_searxng_url() {
        unsafe { std::env::set_var("SEARXNG_URL", "http://127.0.0.1:8080") };
        let backend = SearxngBackend;
        let opts = SearchOptions {
            max_results: 5,
            timeout_secs: 8,
            backend_override: None,
            backend_config: Default::default(),
        };
        let err = backend.search("test", &opts).await.expect_err("blocked");
        assert!(matches!(
            err.kind,
            crate::tools::web::search::error::SearchErrorKind::Hard
        ));
        unsafe { std::env::remove_var("SEARXNG_URL") };
    }

    #[test]
    fn parse_normalizes_score_sorted_results() {
        let data = serde_json::json!({
            "results": [
                {"title": "Low", "url": "https://low.example", "content": "d0", "score": 0.1},
                {"title": "High", "url": "https://high.example", "content": "d1", "score": 0.99},
                {"title": "Mid", "url": "https://mid.example", "content": "d2", "score": 0.5}
            ]
        });
        let rows = parse_searxng_json(&data, 5, "searxng");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].title, "High");
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[1].title, "Mid");
        assert_eq!(rows[2].title, "Low");
        assert_eq!(rows[2].rank, 3);
    }

    #[test]
    fn parse_respects_limit() {
        let data = serde_json::json!({
            "results": [
                {"title": "A", "url": "https://a.example", "content": "", "score": 1.0},
                {"title": "B", "url": "https://b.example", "content": "", "score": 0.9},
                {"title": "C", "url": "https://c.example", "content": "", "score": 0.8}
            ]
        });
        let rows = parse_searxng_json(&data, 2, "searxng");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].title, "A");
        assert_eq!(rows[1].title, "B");
    }

    #[test]
    fn parse_empty_results_is_success_shape() {
        let data = serde_json::json!({ "results": [] });
        assert!(parse_searxng_json(&data, 5, "searxng").is_empty());
    }
}
