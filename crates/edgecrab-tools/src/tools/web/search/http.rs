//! Shared HTTP helpers for search backends.

use edgecrab_security::url_validation::{UrlValidationError, validate_outbound_url};
use edgecrab_types::ToolError;

use super::error::SearchError;

/// Validate outbound URL (SSRF + website blocklist) for any tool.
pub fn validate_url_for_tool(url: &str, tool: &str) -> Result<(), ToolError> {
    validate_outbound_url(url).map_err(|err| map_url_validation_error(url, tool, err))
}

fn map_url_validation_error(url: &str, tool: &str, err: UrlValidationError) -> ToolError {
    match err {
        UrlValidationError::SsrfBlocked(_) => ToolError::PermissionDenied(format!(
            "URL blocked by SSRF policy for tool '{tool}': {url}"
        )),
        UrlValidationError::WebsitePolicyBlocked(msg) => ToolError::PermissionDenied(msg),
        UrlValidationError::Invalid(e) => {
            ToolError::PermissionDenied(format!("URL validation error in '{tool}': {e}"))
        }
    }
}

/// Validate a URL with the SSRF guard + website blocklist before any outbound request.
pub fn validate_search_url(url: &str) -> Result<(), SearchError> {
    validate_outbound_url(url).map_err(|err| match err {
        UrlValidationError::SsrfBlocked(msg) | UrlValidationError::WebsitePolicyBlocked(msg) => {
            SearchError::hard("web_search", msg)
        }
        UrlValidationError::Invalid(e) => SearchError::hard("web_search", e),
    })
}

/// Build a plain reqwest client for trusted JSON API backends.
pub fn build_api_client(timeout_secs: u64) -> Result<reqwest::Client, SearchError> {
    Ok(edgecrab_security::url_safety::build_ssrf_safe_client(
        std::time::Duration::from_secs(timeout_secs.max(1)),
    ))
}

/// Percent-encode a query string for URL embedding.
pub fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            other => {
                let bytes = other.to_string().into_bytes();
                bytes.iter().map(|b| format!("%{:02X}", b)).collect()
            }
        })
        .collect()
}

/// Build Chrome-emulating client for non-DDGS callers (extract/crawl legacy path).
pub fn build_chrome_client(timeout_secs: u64) -> Result<reqwest::Client, SearchError> {
    build_chrome_client_with_headers(timeout_secs, None, None, None)
}

/// Legacy Chrome client — shares TLS/UA pool with DDGS [`fingerprint`] module.
pub fn build_chrome_client_with_headers(
    timeout_secs: u64,
    _referer: Option<&str>,
    _user_agent: Option<&str>,
    _proxy_url: Option<String>,
) -> Result<reqwest::Client, SearchError> {
    build_api_client(timeout_secs)
}

/// Map reqwest errors to SearchError without leaking secrets from URLs.
pub fn map_reqwest_error(backend: &str, err: reqwest::Error) -> SearchError {
    if err.is_timeout() {
        SearchError::timeout(backend, format!("Request timed out: {err}"))
    } else if err.is_connect() || err.is_request() {
        SearchError::network(backend, format!("Network error: {err}"))
    } else {
        SearchError::network(backend, format!("HTTP client error: {err}"))
    }
}

/// Redact API keys from log/error strings (never log secrets).
pub fn redact_secrets(text: &str, secrets: &[&str]) -> String {
    let mut out = text.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            out = out.replace(secret, "[REDACTED]");
        }
    }
    out
}
