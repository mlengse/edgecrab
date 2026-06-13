//! Tavily search backend (~1000 free searches/month).

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::backend_settings::require_api_key;
use crate::tools::web::search::config::{ExtractOptions, SearchOptions};
use crate::tools::web::search::content_extract;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error};

pub struct TavilyBackend;

const TAVILY_ENV_KEYS: &[&str] = &["TAVILY_API_KEY"];

#[async_trait]
impl WebSearchBackend for TavilyBackend {
    fn name(&self) -> &str {
        "tavily"
    }

    fn is_available(&self) -> bool {
        std::env::var("TAVILY_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    fn supports_extract(&self) -> bool {
        true
    }

    fn supports_crawl(&self) -> bool {
        true
    }

    async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let api_key = require_api_key(self.name(), &opts.backend_config, TAVILY_ENV_KEYS)?;
        let client = build_api_client(opts.timeout_secs)?;
        let body = json!({
            "api_key": api_key,
            "query": query,
            "max_results": opts.max_results(),
            "search_depth": "basic",
            "include_answer": false,
        });

        let resp = client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .json(&body)
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
                format!("Tavily HTTP {code}: {safe}"),
            ));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| SearchError::hard(self.name(), format!("Tavily JSON parse error: {e}")))?;

        Ok(data
            .get("results")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .filter_map(|(i, r)| {
                        let title = r.get("title")?.as_str()?.to_string();
                        let url = r.get("url")?.as_str()?.to_string();
                        let snippet = r
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some(SearchResult::new(i + 1, title, url, snippet, self.name()))
                    })
                    .take(opts.max_results())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
    ) -> Result<content_extract::RawExtractPage, content_extract::ExtractHttpError> {
        content_extract::extract_tavily(url, opts.timeout_secs()).await
    }
}
