//! Exa neural search backend (https://exa.ai).

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::backend_settings::require_api_key;
use crate::tools::web::search::config::{ExtractOptions, SearchOptions};
use crate::tools::web::search::content_extract;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error, redact_secrets};

pub struct ExaBackend;

const EXA_ENV_KEYS: &[&str] = &["EXA_API_KEY"];
const EXA_SEARCH_URL: &str = "https://api.exa.ai/search";

/// Normalize Exa JSON into ranked [`SearchResult`] rows (1-indexed rank).
pub(crate) fn parse_exa_json(data: &Value, max: usize, source: &str) -> Vec<SearchResult> {
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
                        .get("highlights")
                        .and_then(|v| v.as_array())
                        .map(|hs| {
                            hs.iter()
                                .filter_map(|h| h.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .filter(|s| !s.is_empty())
                        .or_else(|| r.get("text").and_then(|v| v.as_str()).map(str::to_string))
                        .unwrap_or_default();
                    Some(SearchResult::new(i + 1, title, url, snippet, source))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[async_trait]
impl WebSearchBackend for ExaBackend {
    fn name(&self) -> &str {
        "exa"
    }

    fn is_available(&self) -> bool {
        std::env::var("EXA_API_KEY")
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
        let api_key = require_api_key(self.name(), &opts.backend_config, EXA_ENV_KEYS)?;
        let client = build_api_client(opts.timeout_secs)?;
        let resp = client
            .post(EXA_SEARCH_URL)
            .header("x-api-key", &api_key)
            .header("Content-Type", "application/json")
            .header("x-exa-integration", "edgecrab")
            .json(&json!({
                "query": query,
                "numResults": opts.max_results(),
                "contents": { "highlights": true },
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
                format!("Exa HTTP {code}: {safe}"),
            ));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| SearchError::hard(self.name(), format!("Exa JSON parse error: {e}")))?;
        Ok(parse_exa_json(&data, opts.max_results(), self.name()))
    }

    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
    ) -> Result<content_extract::RawExtractPage, content_extract::ExtractHttpError> {
        content_extract::extract_exa(url, opts.timeout_secs()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_highlights_joined_as_snippet() {
        let data = serde_json::json!({
            "results": [
                {
                    "title": "Exa Hit",
                    "url": "https://exa.example/doc",
                    "highlights": ["First highlight.", "Second highlight."]
                }
            ]
        });
        let rows = parse_exa_json(&data, 5, "exa");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].snippet, "First highlight. Second highlight.");
        assert_eq!(rows[0].rank, 1);
    }

    #[test]
    fn parse_respects_limit() {
        let data = serde_json::json!({
            "results": [
                {"title": "A", "url": "https://a.example", "highlights": ["a"]},
                {"title": "B", "url": "https://b.example", "highlights": ["b"]},
                {"title": "C", "url": "https://c.example", "highlights": ["c"]}
            ]
        });
        assert_eq!(parse_exa_json(&data, 2, "exa").len(), 2);
    }

    #[test]
    fn unavailable_without_api_key() {
        let prev = std::env::var("EXA_API_KEY").ok();
        unsafe { std::env::remove_var("EXA_API_KEY") };
        assert!(!ExaBackend.is_available());
        if let Some(v) = prev {
            unsafe { std::env::set_var("EXA_API_KEY", v) };
        }
    }
}
