//! Parallel.ai search backend (https://parallel.ai).

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::backend_settings::{MAX_SEARCH_RESULTS, require_api_key};
use crate::tools::web::search::config::{ExtractOptions, SearchOptions};
use crate::tools::web::search::content_extract;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error, redact_secrets};

pub struct ParallelBackend;

const PARALLEL_ENV_KEYS: &[&str] = &["PARALLEL_API_KEY"];
const PARALLEL_SEARCH_URL: &str = "https://api.parallel.ai/v1beta/search";
/// Hermes SDK caps Parallel server-side at 20 results per call.
const PARALLEL_MAX_RESULTS: usize = 20;

fn parallel_search_mode() -> String {
    match std::env::var("PARALLEL_SEARCH_MODE")
        .unwrap_or_else(|_| "agentic".into())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "fast" => "fast".into(),
        "one-shot" => "one-shot".into(),
        _ => "agentic".into(),
    }
}

fn parallel_limit(requested: usize) -> usize {
    requested.clamp(1, PARALLEL_MAX_RESULTS.min(MAX_SEARCH_RESULTS))
}

/// Normalize Parallel beta search JSON into ranked [`SearchResult`] rows.
pub(crate) fn parse_parallel_json(data: &Value, max: usize, source: &str) -> Vec<SearchResult> {
    data.get("results")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .take(max)
                .enumerate()
                .filter_map(|(i, r)| {
                    let title = r
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let url = r.get("url").and_then(|v| v.as_str())?.to_string();
                    let snippet = r
                        .get("excerpts")
                        .and_then(|v| v.as_array())
                        .map(|ex| {
                            ex.iter()
                                .filter_map(|e| e.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_default();
                    Some(SearchResult::new(i + 1, title, url, snippet, source))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[async_trait]
impl WebSearchBackend for ParallelBackend {
    fn name(&self) -> &str {
        "parallel"
    }

    fn is_available(&self) -> bool {
        std::env::var("PARALLEL_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    fn supports_extract(&self) -> bool {
        true
    }

    async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let api_key = require_api_key(self.name(), &opts.backend_config, PARALLEL_ENV_KEYS)?;
        let limit = parallel_limit(opts.max_results());
        let mode = parallel_search_mode();
        let client = build_api_client(opts.timeout_secs)?;
        let resp = client
            .post(PARALLEL_SEARCH_URL)
            .header("x-api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&json!({
                "objective": query,
                "search_queries": [query],
                "mode": mode,
                "max_results": limit,
            }))
            .send()
            .await
            .map_err(|e| map_reqwest_error(self.name(), e))?;

        let code = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            let safe = redact_secrets(&text, &[&api_key]);
            return Err(SearchError::from_http_status(
                self.name(),
                code,
                format!("Parallel HTTP {code}: {safe}"),
            ));
        }

        let data: Value = resp.json().await.map_err(|e| {
            SearchError::hard(self.name(), format!("Parallel JSON parse error: {e}"))
        })?;
        Ok(parse_parallel_json(&data, limit, self.name()))
    }

    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
    ) -> Result<content_extract::RawExtractPage, content_extract::ExtractHttpError> {
        content_extract::extract_parallel(url, opts.timeout_secs()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_excerpts_joined_as_snippet() {
        let data = serde_json::json!({
            "results": [
                {
                    "title": "Parallel Hit",
                    "url": "https://parallel.example/page",
                    "excerpts": ["Excerpt one.", "Excerpt two."]
                }
            ]
        });
        let rows = parse_parallel_json(&data, 5, "parallel");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].snippet, "Excerpt one. Excerpt two.");
    }

    #[test]
    fn parallel_limit_caps_at_twenty() {
        assert_eq!(parallel_limit(100), 20);
        assert_eq!(parallel_limit(5), 5);
    }

    #[test]
    fn unavailable_without_api_key() {
        let prev = std::env::var("PARALLEL_API_KEY").ok();
        unsafe { std::env::remove_var("PARALLEL_API_KEY") };
        assert!(!ParallelBackend.is_available());
        if let Some(v) = prev {
            unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
        }
    }
}
