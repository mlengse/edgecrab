//! Brave Search API backend (free tier: 2000 queries/month).

use async_trait::async_trait;
use serde_json::Value;

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::backend_settings::require_api_key;
use crate::tools::web::search::config::SearchOptions;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error, urlencoding_encode};

pub struct BraveBackend;

const BRAVE_ENV_KEYS: &[&str] = &["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"];

#[async_trait]
impl WebSearchBackend for BraveBackend {
    fn name(&self) -> &str {
        "brave"
    }

    fn is_available(&self) -> bool {
        BRAVE_ENV_KEYS.iter().any(|k| {
            std::env::var(k)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        })
    }

    async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let api_key = require_api_key(self.name(), &opts.backend_config, BRAVE_ENV_KEYS)?;
        let client = build_api_client(opts.timeout_secs)?;
        let url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding_encode(query),
            opts.max_results()
        );

        let resp = client
            .get(&url)
            .header("X-Subscription-Token", &api_key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| map_reqwest_error(self.name(), e))?;

        let code = resp.status().as_u16();
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            let safe = crate::tools::web::search::http::redact_secrets(&text, &[&api_key]);
            return Err(SearchError::from_http_status(
                self.name(),
                code,
                format!("Brave Search HTTP {code}: {safe}"),
            ));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| SearchError::hard(self.name(), format!("Brave JSON parse error: {e}")))?;

        Ok(parse_brave_json(&data, opts.max_results(), self.name()))
    }
}

/// Normalize Brave Search JSON into ranked [`SearchResult`] rows (1-indexed rank).
pub(crate) fn parse_brave_json(data: &Value, max: usize, source: &str) -> Vec<SearchResult> {
    data.get("web")
        .and_then(|w| w.get("results"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter_map(|(i, r)| {
                    let title = r.get("title")?.as_str()?.to_string();
                    let url = r.get("url")?.as_str()?.to_string();
                    let snippet = r
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(SearchResult::new(i + 1, title, url, snippet, source))
                })
                .take(max)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::web::search::http::redact_secrets;

    #[test]
    fn api_key_redacted_from_error_text() {
        let key = "BSA-secret-key-12345";
        let msg = format!("Brave Search HTTP 401: invalid token {key}");
        let redacted = redact_secrets(&msg, &[key]);
        assert!(!redacted.contains(key));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn parse_normalizes_brave_free_shape() {
        let data = serde_json::json!({
            "web": {
                "results": [
                    {"title": "A", "url": "https://a.example.com", "description": "desc A"},
                    {"title": "B", "url": "https://b.example.com", "description": "desc B"},
                    {"title": "C", "url": "https://c.example.com", "description": "desc C"}
                ]
            }
        });
        let rows = parse_brave_json(&data, 5, "brave");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].title, "A");
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[0].snippet, "desc A");
        assert_eq!(rows[2].rank, 3);
    }

    #[test]
    fn parse_respects_limit() {
        let data = serde_json::json!({
            "web": {
                "results": [
                    {"title": "A", "url": "https://a.example", "description": ""},
                    {"title": "B", "url": "https://b.example", "description": ""},
                    {"title": "C", "url": "https://c.example", "description": ""}
                ]
            }
        });
        assert_eq!(parse_brave_json(&data, 2, "brave").len(), 2);
    }

    #[test]
    fn unavailable_without_api_key() {
        let prev = std::env::var("BRAVE_API_KEY")
            .ok()
            .or_else(|| std::env::var("BRAVE_SEARCH_API_KEY").ok());
        unsafe {
            std::env::remove_var("BRAVE_API_KEY");
            std::env::remove_var("BRAVE_SEARCH_API_KEY");
        }
        assert!(!BraveBackend.is_available());
        if let Some(v) = prev {
            unsafe { std::env::set_var("BRAVE_API_KEY", v) };
        }
    }
}
