//! DRY HTTP helpers for Firecrawl / Tavily **crawl** APIs (distinct from single-page extract).

use reqwest::Method;
use serde_json::{Value, json};
use tokio::time::{Duration, sleep};

use super::content_extract::{ExtractHttpError, firecrawl_api, tavily_api};

const CRAWL_POLL_INTERVAL_SECS: u64 = 1;
const CRAWL_MAX_POLL_ATTEMPTS: usize = 45;

/// Start a Firecrawl crawl job; returns the job id.
pub async fn firecrawl_start_crawl(
    payload: Value,
    timeout_secs: u64,
) -> Result<String, ExtractHttpError> {
    let started = firecrawl_api(Method::POST, "crawl", Some(payload), timeout_secs).await?;
    started
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| ExtractHttpError::hard("Firecrawl crawl did not return a job id."))
}

/// Poll until Firecrawl crawl completes; returns the final status payload.
pub async fn firecrawl_wait_crawl(
    job_id: &str,
    timeout_secs: u64,
) -> Result<Value, ExtractHttpError> {
    for attempt in 0..CRAWL_MAX_POLL_ATTEMPTS {
        let status =
            firecrawl_api(Method::GET, &format!("crawl/{job_id}"), None, timeout_secs).await?;
        match status.get("status").and_then(|v| v.as_str()) {
            Some("completed") => return Ok(status),
            Some("failed") => {
                let failure = status
                    .get("error")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        status["data"].as_array().and_then(|data| {
                            data.iter()
                                .find_map(|value| value["metadata"]["error"].as_str())
                        })
                    })
                    .unwrap_or("Firecrawl crawl failed.");
                return Err(ExtractHttpError::hard(failure));
            }
            _ => {
                if attempt + 1 >= CRAWL_MAX_POLL_ATTEMPTS {
                    return Err(ExtractHttpError::hard(
                        "Firecrawl crawl timed out waiting for completion.",
                    ));
                }
                sleep(Duration::from_secs(CRAWL_POLL_INTERVAL_SECS)).await;
            }
        }
    }
    Err(ExtractHttpError::hard(
        "Firecrawl crawl timed out waiting for completion.",
    ))
}

/// Fetch next page of a paginated Firecrawl crawl response (when `next` URL present).
pub async fn firecrawl_fetch_crawl_page(
    next_url: &str,
    timeout_secs: u64,
) -> Result<Value, ExtractHttpError> {
    firecrawl_api(Method::GET, next_url, None, timeout_secs).await
}

/// Run Tavily crawl API; returns raw JSON (contains `results` array).
pub async fn tavily_crawl(payload: Value, timeout_secs: u64) -> Result<Value, ExtractHttpError> {
    tavily_api("crawl", payload, timeout_secs).await
}

/// Build Firecrawl crawl start payload (shared shape for web_crawl tool).
pub fn firecrawl_crawl_payload(
    start_url: &str,
    max_pages: usize,
    max_depth: usize,
    same_path_only: bool,
    include_paths: Option<Vec<String>>,
    instructions: Option<&str>,
) -> Value {
    let mut payload = json!({
        "url": start_url,
        "limit": max_pages,
        "maxDiscoveryDepth": max_depth,
        "allowExternalLinks": false,
        "allowSubdomains": false,
        "crawlEntireDomain": !same_path_only,
        "scrapeOptions": {
            "formats": ["markdown", "links"],
            "onlyMainContent": true,
        },
    });
    if let Some(instructions) = instructions {
        payload["prompt"] = Value::String(instructions.to_string());
    }
    if let Some(paths) = include_paths {
        payload["includePaths"] =
            Value::Array(paths.into_iter().map(serde_json::Value::String).collect());
    }
    payload
}

/// Build Tavily crawl payload.
pub fn tavily_crawl_payload(url: &str, max_pages: usize, instructions: Option<&str>) -> Value {
    let mut payload = json!({
        "url": url,
        "limit": max_pages,
        "extract_depth": "advanced",
    });
    if let Some(instructions) = instructions {
        payload["instructions"] = Value::String(instructions.to_string());
    }
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firecrawl_payload_includes_scrape_options() {
        let p = firecrawl_crawl_payload("https://example.com", 5, 2, true, None, None);
        assert_eq!(p["url"], "https://example.com");
        assert_eq!(p["limit"], 5);
        assert!(p["scrapeOptions"]["formats"].is_array());
    }

    #[test]
    fn tavily_payload_includes_limit() {
        let p = tavily_crawl_payload("https://example.com", 8, Some("find docs"));
        assert_eq!(p["limit"], 8);
        assert_eq!(p["instructions"], "find docs");
    }
}
