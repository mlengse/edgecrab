//! xAI Grok agentic web search backend (https://docs.x.ai/developers/tools/web-search).
//!
//! Uses the Responses API with the server-side `web_search` tool. Requires `XAI_API_KEY`
//! (Hermes also supports OAuth — env key only for now).

use async_trait::async_trait;
use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;

use crate::tools::web::search::backend::{SearchResult, WebSearchBackend};
use crate::tools::web::search::backend_settings::require_api_key;
use crate::tools::web::search::config::SearchOptions;
use crate::tools::web::search::error::SearchError;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error, redact_secrets};

pub struct XaiBackend;

const XAI_ENV_KEYS: &[&str] = &["XAI_API_KEY"];
const DEFAULT_MODEL: &str = "grok-4";
const DEFAULT_BASE: &str = "https://api.x.ai/v1";

static JSON_BLOCK_RE: OnceLock<Regex> = OnceLock::new();

fn xai_base_url(opts: &SearchOptions) -> String {
    opts.backend_config
        .endpoint
        .as_deref()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::env::var("XAI_BASE_URL")
                .ok()
                .map(|v| v.trim().trim_end_matches('/').to_string())
                .filter(|v| !v.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_BASE.to_string())
}

fn build_prompt(query: &str, limit: usize) -> String {
    format!(
        "Use the web_search tool to find current information for the query below, \
         then respond with ONLY a single JSON object — no prose, no markdown fences — \
         matching this exact schema:\n\n\
         {{\"results\": [{{\"title\": \"string\", \"url\": \"string\", \
         \"description\": \"1-2 sentence summary\"}}]}}\n\n\
         Return at most {limit} results, ordered by relevance, with absolute https:// URLs. \
         If no usable results exist, return {{\"results\": []}}.\n\n\
         Query: {query}"
    )
}

/// Parse Grok JSON block from Responses API output.
pub(crate) fn parse_xai_response(data: &Value, max: usize) -> Vec<SearchResult> {
    let text_blocks = collect_output_text(data);
    for block in &text_blocks {
        if let Some(rows) = try_parse_json_results(block, max) {
            return rows;
        }
    }

    // Fallback: citations list
    data.get("citations")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .take(max)
                .enumerate()
                .filter_map(|(i, u)| {
                    let url = u.as_str()?.to_string();
                    if url.trim().is_empty() {
                        return None;
                    }
                    Some(SearchResult::new(
                        i + 1,
                        String::new(),
                        url,
                        String::new(),
                        "xai",
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn collect_output_text(data: &Value) -> Vec<String> {
    let mut blocks = Vec::new();
    let Some(output) = data.get("output").and_then(|v| v.as_array()) else {
        return blocks;
    };
    for item in output {
        if item.get("type").and_then(|v| v.as_str()) != Some("message") {
            continue;
        }
        let Some(content) = item.get("content").and_then(|v| v.as_array()) else {
            continue;
        };
        for chunk in content {
            if chunk.get("type").and_then(|v| v.as_str()) == Some("output_text")
                && let Some(text) = chunk.get("text").and_then(|v| v.as_str())
                && !text.trim().is_empty()
            {
                blocks.push(text.to_string());
            }
        }
    }
    blocks
}

fn try_parse_json_results(text: &str, max: usize) -> Option<Vec<SearchResult>> {
    let re = JSON_BLOCK_RE.get_or_init(|| Regex::new(r"\{[\s\S]*\}").expect("valid regex"));
    let candidates = std::iter::once(text.to_string())
        .chain(re.find(text).into_iter().map(|m| m.as_str().to_string()));
    for candidate in candidates {
        let Ok(parsed) = serde_json::from_str::<Value>(&candidate) else {
            continue;
        };
        let Some(results) = parsed.get("results").and_then(|v| v.as_array()) else {
            continue;
        };
        let mut rows = Vec::new();
        for row in results.iter().take(max) {
            let url = row.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
            if url.is_empty() {
                continue;
            }
            rows.push(SearchResult::new(
                rows.len() + 1,
                row.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string(),
                url.to_string(),
                row.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string(),
                "xai",
            ));
        }
        if !rows.is_empty() {
            return Some(rows);
        }
    }
    None
}

#[async_trait]
impl WebSearchBackend for XaiBackend {
    fn name(&self) -> &str {
        "xai"
    }

    fn is_available(&self) -> bool {
        std::env::var("XAI_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    async fn search(
        &self,
        query: &str,
        opts: &SearchOptions,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let api_key = require_api_key(self.name(), &opts.backend_config, XAI_ENV_KEYS)?;
        let base = xai_base_url(opts);
        let limit = opts.max_results();
        let client = build_api_client(opts.timeout_secs.max(30))?;
        let body = json!({
            "model": DEFAULT_MODEL,
            "input": [{"role": "user", "content": build_prompt(query, limit)}],
            "tools": [{"type": "web_search"}],
            "include": ["no_inline_citations"],
        });

        let resp = client
            .post(format!("{base}/responses"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
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
                format!("xAI HTTP {code}: {safe}"),
            ));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| SearchError::hard(self.name(), format!("xAI JSON parse error: {e}")))?;

        if let Some(err) = data.get("error").and_then(|v| v.as_object()) {
            let msg = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown xAI error");
            return Err(SearchError::hard(self.name(), format!("xAI error: {msg}")));
        }

        Ok(parse_xai_response(&data, limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_results_from_output_text() {
        let data = serde_json::json!({
            "output": [{
                "type": "message",
                "content": [{
                    "type": "output_text",
                    "text": "{\"results\": [{\"title\": \"EdgeCrab\", \"url\": \"https://edgecrab.com\", \"description\": \"Agent\"}]}"
                }]
            }]
        });
        let rows = parse_xai_response(&data, 5);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "EdgeCrab");
        assert_eq!(rows[0].url, "https://edgecrab.com");
    }

    #[test]
    fn parse_falls_back_to_citations() {
        let data = serde_json::json!({
            "citations": ["https://a.example", "https://b.example"]
        });
        let rows = parse_xai_response(&data, 5);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].url, "https://a.example");
    }

    #[test]
    fn unavailable_without_api_key() {
        let prev = std::env::var("XAI_API_KEY").ok();
        unsafe { std::env::remove_var("XAI_API_KEY") };
        assert!(!XaiBackend.is_available());
        if let Some(v) = prev {
            unsafe { std::env::set_var("XAI_API_KEY", v) };
        }
    }
}
