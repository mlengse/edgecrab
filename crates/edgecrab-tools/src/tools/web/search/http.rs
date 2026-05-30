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

/// Build Chrome-emulating client for DDGS metasearch (cookie jar + random UA).
pub fn build_chrome_client(timeout_secs: u64) -> Result<wreq::Client, SearchError> {
    build_chrome_client_with_headers(timeout_secs, None, None)
}

/// Chrome TLS client with optional Referer and User-Agent (Python `ddgs` uses random impersonation).
pub fn build_chrome_client_with_headers(
    timeout_secs: u64,
    referer: Option<&str>,
    user_agent: Option<&str>,
) -> Result<wreq::Client, SearchError> {
    use wreq::{
        EmulationProvider, SslCurve,
        header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue, REFERER, USER_AGENT},
        tls::{AlpnProtos, TlsConfig, TlsVersion},
    };

    let tls = TlsConfig::builder()
        .min_tls_version(TlsVersion::TLS_1_2)
        .max_tls_version(TlsVersion::TLS_1_3)
        .cipher_list(concat!(
            "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:",
            "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256:TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256:",
            "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384:TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384:",
            "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256:",
            "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256:",
            "TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA:TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA:",
            "TLS_RSA_WITH_AES_128_GCM_SHA256:TLS_RSA_WITH_AES_256_GCM_SHA384:",
            "TLS_RSA_WITH_AES_128_CBC_SHA:TLS_RSA_WITH_AES_256_CBC_SHA"
        ))
        .sigalgs_list(concat!(
            "ecdsa_secp256r1_sha256:rsa_pss_rsae_sha256:rsa_pkcs1_sha256:",
            "ecdsa_secp384r1_sha384:rsa_pss_rsae_sha384:rsa_pkcs1_sha384:",
            "rsa_pss_rsae_sha512:rsa_pkcs1_sha512"
        ))
        .curves(vec![
            SslCurve::X25519,
            SslCurve::SECP256R1,
            SslCurve::SECP384R1,
        ])
        .alpn_protos(AlpnProtos::ALL)
        .grease_enabled(true)
        .permute_extensions(true)
        .enable_ech_grease(true)
        .pre_shared_key(true)
        .enable_ocsp_stapling(true)
        .build();

    let ua: &str = match user_agent {
        Some(ua) => ua,
        None => pick_random_user_agent(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(ua)
            .map_err(|e| SearchError::hard("web_search", format!("Invalid User-Agent: {e}")))?,
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    if let Some(r) = referer {
        headers.insert(
            REFERER,
            HeaderValue::from_str(r)
                .map_err(|e| SearchError::hard("web_search", format!("Invalid Referer: {e}")))?,
        );
        headers.insert(
            "Sec-Fetch-User",
            HeaderValue::from_static("?1"),
        );
    }

    let provider = EmulationProvider::builder()
        .tls_config(tls)
        .default_headers(headers)
        .build();

    let mut builder = wreq::Client::builder()
        .emulation(provider)
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(timeout_secs.max(1)));

    if let Some(proxy_url) = edgecrab_security::proxy::resolve_proxy_url(None)
        && let Ok(proxy) = wreq::Proxy::all(&proxy_url)
    {
        builder = builder.proxy(proxy);
    }

    builder
        .build()
        .map_err(|e| SearchError::hard("web_search", format!("Failed to build Chrome client: {e}")))
}

const CHROME_USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
];

fn pick_random_user_agent() -> &'static str {
    use rand::Rng;
    let idx = rand::rng().random_range(0..CHROME_USER_AGENTS.len());
    CHROME_USER_AGENTS[idx]
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

/// Legacy helper for extract/crawl module.
pub fn validate_url_legacy(url: &str, tool: &str) -> Result<(), ToolError> {
    validate_url_for_tool(url, tool)
}
