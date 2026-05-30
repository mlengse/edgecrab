//! Shared HTTP extract helpers for all paid extract providers (Hermes extract parity).
//!
//! Search backends implement [`super::backend::WebSearchBackend`]; extract-capable
//! providers share this module to avoid duplicating API calls in `extract_crawl`.

use reqwest::Method;
use serde_json::{Value, json};

use crate::tools::web::search::backend_settings::resolve_api_key;
use crate::tools::web::search::config::load_web_search_config_from_disk;
use crate::tools::web::search::http::{build_api_client, map_reqwest_error, redact_secrets};

const EXA_ENV_KEYS: &[&str] = &["EXA_API_KEY"];
const PARALLEL_ENV_KEYS: &[&str] = &["PARALLEL_API_KEY"];
const FIRECRAWL_ENV_KEYS: &[&str] = &["FIRECRAWL_API_KEY"];
const TAVILY_ENV_KEYS: &[&str] = &["TAVILY_API_KEY"];
const EXA_CONTENTS_URL: &str = "https://api.exa.ai/contents";
const PARALLEL_EXTRACT_URL: &str = "https://api.parallel.ai/v1beta/extract";
const FIRECRAWL_API_BASE: &str = "https://api.firecrawl.dev/v2";
const TAVILY_API_BASE: &str = "https://api.tavily.com";

/// HTTP-layer error for extract API calls (maps to `BackendError` in extract_crawl).
#[derive(Debug, Clone)]
pub struct ExtractHttpError {
    pub status: Option<u16>,
    pub message: String,
}

impl ExtractHttpError {
    pub fn hard(message: impl Into<String>) -> Self {
        Self {
            status: Some(0),
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self {
            status: None,
            message: message.into(),
        }
    }

    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self {
            status: Some(status),
            message: message.into(),
        }
    }

    pub fn is_transient(&self) -> bool {
        matches!(self.status, None | Some(402 | 429 | 500 | 502 | 503 | 504))
    }
}

/// Raw page content before truncation / `ExtractedDocument` assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawExtractPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub extractor: &'static str,
    pub content_type: Option<String>,
    pub content_format: Option<String>,
    pub meta_description: Option<String>,
}

fn backend_config(name: &str) -> crate::config_ref::WebSearchBackendConfigRef {
    let disk = load_web_search_config_from_disk();
    crate::tools::web::search::backend_settings::lookup_backend_config(&disk.backends, name)
}

fn resolve_exa_key() -> Result<String, ExtractHttpError> {
    resolve_api_key(&backend_config("exa"), EXA_ENV_KEYS).ok_or_else(|| {
        ExtractHttpError::hard(
            "EXA_API_KEY is not set. Set web_search.backends.exa.api_key or EXA_API_KEY.",
        )
    })
}

fn resolve_parallel_key() -> Result<String, ExtractHttpError> {
    resolve_api_key(&backend_config("parallel"), PARALLEL_ENV_KEYS).ok_or_else(|| {
        ExtractHttpError::hard(
            "PARALLEL_API_KEY is not set. Set web_search.backends.parallel.api_key or PARALLEL_API_KEY.",
        )
    })
}

fn resolve_firecrawl_key() -> Result<String, ExtractHttpError> {
    resolve_api_key(&backend_config("firecrawl"), FIRECRAWL_ENV_KEYS).ok_or_else(|| {
        ExtractHttpError::hard(
            "FIRECRAWL_API_KEY is not set. Set web_search.backends.firecrawl.api_key or FIRECRAWL_API_KEY.",
        )
    })
}

fn resolve_tavily_key() -> Result<String, ExtractHttpError> {
    resolve_api_key(&backend_config("tavily"), TAVILY_ENV_KEYS).ok_or_else(|| {
        ExtractHttpError::hard(
            "TAVILY_API_KEY is not set. Set web_search.backends.tavily.api_key or TAVILY_API_KEY.",
        )
    })
}

fn firecrawl_metadata_text(metadata: &Value, key: &str) -> Option<String> {
    match &metadata[key] {
        Value::String(value) => Some(value.clone()),
        Value::Array(values) => values.iter().find_map(|value| {
            value
                .as_str()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        }),
        _ => None,
    }
}

/// Parse Exa `/contents` JSON into a single-page result.
pub(crate) fn parse_exa_contents(data: &Value, fallback_url: &str) -> Option<RawExtractPage> {
    let row = data.get("results").and_then(|v| v.as_array())?.first()?;
    let url = row
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or(fallback_url)
        .to_string();
    if url.is_empty() {
        return None;
    }
    let title = row
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let content = row
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    Some(RawExtractPage {
        url,
        title,
        content,
        extractor: "exa",
        content_type: None,
        content_format: Some("text".into()),
        meta_description: None,
    })
}

/// Parse Parallel `/v1beta/extract` JSON (first successful result).
pub(crate) fn parse_parallel_extract(data: &Value, fallback_url: &str) -> Option<RawExtractPage> {
    if let Some(row) = data
        .get("results")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
    {
        let url = row
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or(fallback_url)
            .to_string();
        if url.is_empty() {
            return None;
        }
        let title = row
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let content = row
            .get("full_content")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| {
                row.get("excerpts")
                    .and_then(|v| v.as_array())
                    .map(|ex| {
                        ex.iter()
                            .filter_map(|e| e.as_str())
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    })
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_default();
        return Some(RawExtractPage {
            url,
            title,
            content,
            extractor: "parallel",
            content_type: None,
            content_format: Some("text".into()),
            meta_description: None,
        });
    }

    // Per-URL errors: no successful results row.
    None
}

/// Extract one URL via Exa Contents API.
pub async fn extract_exa(url: &str, timeout_secs: u64) -> Result<RawExtractPage, ExtractHttpError> {
    let api_key = resolve_exa_key()?;
    let client = build_api_client(timeout_secs).map_err(|e| ExtractHttpError::hard(e.message))?;
    let resp = client
        .post(EXA_CONTENTS_URL)
        .header("x-api-key", &api_key)
        .header("Content-Type", "application/json")
        .header("x-exa-integration", "edgecrab")
        .json(&json!({
            "urls": [url],
            "text": true,
        }))
        .send()
        .await
        .map_err(|e| {
            let se = map_reqwest_error("exa", e);
            ExtractHttpError::network(se.message)
        })?;

    let code = resp.status().as_u16();
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        let safe = redact_secrets(&text, &[&api_key]);
        return Err(ExtractHttpError::api(
            code,
            format!("Exa extract HTTP {code}: {safe}"),
        ));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| ExtractHttpError::hard(format!("Exa extract JSON parse error: {e}")))?;

    parse_exa_contents(&data, url)
        .ok_or_else(|| ExtractHttpError::hard("Exa extraction returned no document for URL."))
}

/// Extract one URL via Parallel beta extract API.
pub async fn extract_parallel(
    url: &str,
    timeout_secs: u64,
) -> Result<RawExtractPage, ExtractHttpError> {
    let api_key = resolve_parallel_key()?;
    let client = build_api_client(timeout_secs).map_err(|e| ExtractHttpError::hard(e.message))?;
    let resp = client
        .post(PARALLEL_EXTRACT_URL)
        .header("x-api-key", &api_key)
        .header("Content-Type", "application/json")
        .json(&json!({
            "urls": [url],
            "full_content": true,
        }))
        .send()
        .await
        .map_err(|e| ExtractHttpError::network(map_reqwest_error("parallel", e).message))?;

    let code = resp.status().as_u16();
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        let safe = redact_secrets(&text, &[&api_key]);
        return Err(ExtractHttpError::api(
            code,
            format!("Parallel extract HTTP {code}: {safe}"),
        ));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| ExtractHttpError::hard(format!("Parallel extract JSON parse error: {e}")))?;

    if let Some(page) = parse_parallel_extract(&data, url) {
        return Ok(page);
    }

    let err_msg = data
        .get("errors")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|err| {
            err.get("content")
                .or_else(|| err.get("error"))
                .or_else(|| err.get("error_type"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("Parallel extraction returned no document.");

    Err(ExtractHttpError::hard(err_msg))
}

/// Parse Firecrawl scrape/crawl document JSON.
pub(crate) fn parse_firecrawl_document(
    value: &Value,
    fallback_url: &str,
) -> Option<RawExtractPage> {
    let metadata = value.get("metadata").unwrap_or(value);
    let url = metadata
        .get("url")
        .and_then(|v| v.as_str())
        .or_else(|| metadata.get("sourceURL").and_then(|v| v.as_str()))
        .or_else(|| value.get("url").and_then(|v| v.as_str()))
        .unwrap_or(fallback_url)
        .to_string();
    if url.is_empty() {
        return None;
    }
    let title = firecrawl_metadata_text(metadata, "title")
        .or_else(|| {
            value
                .get("title")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let content_format = if value.get("markdown").is_some() {
        "markdown"
    } else if value.get("html").is_some() || value.get("rawHtml").is_some() {
        "html"
    } else {
        "text"
    };
    let raw = value
        .get("markdown")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("html").and_then(|v| v.as_str()))
        .or_else(|| value.get("rawHtml").and_then(|v| v.as_str()))
        .or_else(|| value.get("text").and_then(|v| v.as_str()))
        .unwrap_or_default()
        .to_string();
    Some(RawExtractPage {
        url,
        title,
        content: raw,
        extractor: "firecrawl",
        content_type: metadata
            .get("contentType")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        content_format: Some(content_format.into()),
        meta_description: firecrawl_metadata_text(metadata, "description")
            .or_else(|| {
                value
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .filter(|v| !v.is_empty()),
    })
}

/// Parse Tavily extract result row.
pub(crate) fn parse_tavily_document(value: &Value, fallback_url: &str) -> Option<RawExtractPage> {
    let url = value
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or(fallback_url)
        .to_string();
    if url.is_empty() {
        return None;
    }
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let content = value
        .get("raw_content")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("content").and_then(|v| v.as_str()))
        .unwrap_or_default()
        .to_string();
    Some(RawExtractPage {
        url,
        title,
        content,
        extractor: "tavily",
        content_type: Some("text/html".into()),
        content_format: Some("text".into()),
        meta_description: value
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .filter(|v| !v.is_empty()),
    })
}

/// Low-level Firecrawl API call (scrape, crawl, status, …).
pub async fn firecrawl_api(
    method: Method,
    path_or_url: &str,
    payload: Option<Value>,
    timeout_secs: u64,
) -> Result<Value, ExtractHttpError> {
    let api_key = resolve_firecrawl_key()?;
    let client = build_api_client(timeout_secs).map_err(|e| ExtractHttpError::hard(e.message))?;
    let url = if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        path_or_url.to_string()
    } else {
        format!(
            "{FIRECRAWL_API_BASE}/{}",
            path_or_url.trim_start_matches('/')
        )
    };
    let mut req = client
        .request(method, &url)
        .header("Authorization", format!("Bearer {api_key}"));
    if let Some(body) = payload {
        req = req.header("Content-Type", "application/json").json(&body);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| ExtractHttpError::network(map_reqwest_error("firecrawl", e).message))?;
    let code = resp.status().as_u16();
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        let safe = redact_secrets(&text, &[&api_key]);
        return Err(ExtractHttpError::api(
            code,
            format!("Firecrawl HTTP {code}: {safe}"),
        ));
    }
    resp.json()
        .await
        .map_err(|e| ExtractHttpError::hard(format!("Firecrawl JSON parse error: {e}")))
}

/// Low-level Tavily API call.
pub async fn tavily_api(
    endpoint: &str,
    payload: Value,
    timeout_secs: u64,
) -> Result<Value, ExtractHttpError> {
    let api_key = resolve_tavily_key()?;
    let client = build_api_client(timeout_secs).map_err(|e| ExtractHttpError::hard(e.message))?;
    let url = format!("{TAVILY_API_BASE}/{}", endpoint.trim_start_matches('/'));
    let body = match payload {
        Value::Object(mut map) => {
            map.insert("api_key".into(), Value::String(api_key.clone()));
            Value::Object(map)
        }
        _ => return Err(ExtractHttpError::hard("Invalid Tavily payload shape.")),
    };
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ExtractHttpError::network(map_reqwest_error("tavily", e).message))?;
    let code = resp.status().as_u16();
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        let safe = redact_secrets(&text, &[&api_key]);
        return Err(ExtractHttpError::api(
            code,
            format!("Tavily HTTP {code}: {safe}"),
        ));
    }
    resp.json()
        .await
        .map_err(|e| ExtractHttpError::hard(format!("Tavily JSON parse error: {e}")))
}

/// Extract one URL via Firecrawl scrape API.
pub async fn extract_firecrawl(
    url: &str,
    timeout_secs: u64,
) -> Result<RawExtractPage, ExtractHttpError> {
    let data = firecrawl_api(
        Method::POST,
        "scrape",
        Some(json!({
            "url": url,
            "formats": ["markdown"],
            "onlyMainContent": true,
        })),
        timeout_secs,
    )
    .await?;
    parse_firecrawl_document(data.get("data").unwrap_or(&data), url)
        .ok_or_else(|| ExtractHttpError::hard("Firecrawl extraction returned no document."))
}

/// Extract one URL via Tavily extract API.
pub async fn extract_tavily(
    url: &str,
    timeout_secs: u64,
) -> Result<RawExtractPage, ExtractHttpError> {
    let data = tavily_api(
        "extract",
        json!({
            "urls": [url],
            "include_images": false,
        }),
        timeout_secs,
    )
    .await?;
    if let Some(page) = data
        .get("results")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|row| parse_tavily_document(row, url))
    {
        return Ok(page);
    }
    let failure = data
        .get("failed_results")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|row| row.get("error").and_then(|v| v.as_str()))
        .unwrap_or("Tavily extraction returned no document.");
    Err(ExtractHttpError::hard(failure))
}

/// Hermes-aligned auto extract chain order (paid APIs → native).
pub const EXTRACT_AUTO_CHAIN: &[&str] = &["firecrawl", "parallel", "tavily", "exa"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exa_contents_text_field() {
        let data = serde_json::json!({
            "results": [{
                "url": "https://exa.example/page",
                "title": "Exa Page",
                "text": "Full page body."
            }]
        });
        let page = parse_exa_contents(&data, "https://fallback.example").expect("parsed");
        assert_eq!(page.url, "https://exa.example/page");
        assert_eq!(page.title, "Exa Page");
        assert_eq!(page.content, "Full page body.");
        assert_eq!(page.extractor, "exa");
    }

    #[test]
    fn parse_parallel_prefers_full_content() {
        let data = serde_json::json!({
            "results": [{
                "url": "https://parallel.example/doc",
                "title": "Parallel Doc",
                "full_content": "Full text here.",
                "excerpts": ["snippet"]
            }]
        });
        let page = parse_parallel_extract(&data, "https://fallback.example").expect("parsed");
        assert_eq!(page.content, "Full text here.");
        assert_eq!(page.extractor, "parallel");
    }

    #[test]
    fn parse_parallel_falls_back_to_excerpts() {
        let data = serde_json::json!({
            "results": [{
                "url": "https://parallel.example/doc",
                "title": "T",
                "excerpts": ["Part one.", "Part two."]
            }]
        });
        let page = parse_parallel_extract(&data, "https://fallback.example").expect("parsed");
        assert!(page.content.contains("Part one."));
        assert!(page.content.contains("Part two."));
    }

    #[test]
    fn extract_http_error_transient_codes() {
        assert!(ExtractHttpError::api(429, "rate limit").is_transient());
        assert!(ExtractHttpError::api(503, "down").is_transient());
        assert!(!ExtractHttpError::api(404, "missing").is_transient());
        assert!(ExtractHttpError::network("timeout").is_transient());
    }

    #[test]
    fn parse_firecrawl_markdown_document() {
        let data = serde_json::json!({
            "markdown": "# Title\nBody",
            "metadata": { "url": "https://fc.example", "title": "FC Title" }
        });
        let page = parse_firecrawl_document(&data, "https://fallback.example").expect("parsed");
        assert_eq!(page.extractor, "firecrawl");
        assert_eq!(page.content_format.as_deref(), Some("markdown"));
    }

    #[test]
    fn parse_tavily_raw_content() {
        let data = serde_json::json!({
            "url": "https://tv.example",
            "title": "TV",
            "raw_content": "Article text"
        });
        let page = parse_tavily_document(&data, "https://fallback.example").expect("parsed");
        assert_eq!(page.content, "Article text");
        assert_eq!(page.extractor, "tavily");
    }

    #[test]
    fn extract_auto_chain_matches_hermes_order() {
        assert_eq!(
            EXTRACT_AUTO_CHAIN,
            &["firecrawl", "parallel", "tavily", "exa"]
        );
    }
}
