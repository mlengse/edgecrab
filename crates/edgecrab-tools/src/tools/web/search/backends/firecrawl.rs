//! Firecrawl search backend (premium search + scrape-ready results).

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::backend_settings::resolve_api_key;
use crate::tools::web::search::config::{ExtractOptions, SearchOptions};
use crate::tools::web::search::content_extract;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error};

pub struct FirecrawlBackend;

const FIRECRAWL_ENV_KEYS: &[&str] = &["FIRECRAWL_API_KEY"];

fn firecrawl_api_key(opts: &SearchOptions) -> Option<String> {
    resolve_api_key(&opts.backend_config, FIRECRAWL_ENV_KEYS)
}

fn normalize_results(data: &Value, max: usize, source: &str) -> Vec<SearchResult> {
    let array = data
        .get("data")
        .and_then(|d| d.get("web"))
        .and_then(|v| v.as_array())
        .or_else(|| data.get("data").and_then(|v| v.as_array()));

    array
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, value)| {
            let metadata = value.get("metadata").unwrap_or(value);
            let title = value
                .get("title")
                .and_then(|v| v.as_str())
                .or_else(|| metadata.get("title").and_then(|v| v.as_str()))
                .unwrap_or_default()
                .to_string();
            let url = value
                .get("url")
                .and_then(|v| v.as_str())
                .or_else(|| metadata.get("url").and_then(|v| v.as_str()))
                .or_else(|| metadata.get("sourceURL").and_then(|v| v.as_str()))?
                .to_string();
            let snippet = value
                .get("description")
                .and_then(|v| v.as_str())
                .or_else(|| metadata.get("description").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            Some(SearchResult::new(i + 1, title, url, snippet, source))
        })
        .take(max)
        .collect()
}

#[async_trait]
impl WebSearchBackend for FirecrawlBackend {
    fn name(&self) -> &str {
        "firecrawl"
    }

    fn is_available(&self) -> bool {
        std::env::var("FIRECRAWL_API_KEY")
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
        let api_key = firecrawl_api_key(opts)
            .ok_or_else(|| SearchError::hard(self.name(), "FIRECRAWL_API_KEY is not set."))?;
        let client = build_api_client(opts.timeout_secs)?;
        let resp = client
            .post("https://api.firecrawl.dev/v2/search")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&json!({
                "query": query,
                "limit": opts.max_results(),
                "ignoreInvalidURLs": true,
            }))
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
                format!("Firecrawl HTTP {code}: {safe}"),
            ));
        }

        let data: Value = resp.json().await.map_err(|e| {
            SearchError::hard(self.name(), format!("Firecrawl JSON parse error: {e}"))
        })?;
        Ok(normalize_results(&data, opts.max_results(), self.name()))
    }

    async fn extract(
        &self,
        url: &str,
        opts: &ExtractOptions,
    ) -> Result<content_extract::RawExtractPage, content_extract::ExtractHttpError> {
        content_extract::extract_firecrawl(url, opts.timeout_secs()).await
    }
}
