//! Inference base URL validation (Hermes `_validate_nous_inference_url_from_network`).

use std::env;

pub const DEFAULT_NOUS_INFERENCE: &str = "https://inference-api.nousresearch.com/v1";

const ALLOWED_HOSTS: &[&str] = &["inference-api.nousresearch.com"];

/// Trusted user override from the environment (Hermes `NOUS_INFERENCE_BASE_URL`).
pub fn inference_url_from_env() -> Option<String> {
    env::var("NOUS_INFERENCE_BASE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
}

/// Validate Portal-returned inference URL; fall back when missing or poisoned.
pub fn validate_nous_inference_url_from_network(url: Option<&str>, fallback: &str) -> String {
    if let Some(env) = inference_url_from_env() {
        return env;
    }
    let Some(raw) = url.map(str::trim).filter(|s| !s.is_empty()) else {
        return normalize_fallback(fallback);
    };
    let Ok(parsed) = url::Url::parse(raw) else {
        tracing::warn!("nous: refusing malformed inference URL from Portal response");
        return normalize_fallback(fallback);
    };
    if parsed.scheme() != "https" {
        tracing::warn!(
            "nous: refusing non-https inference URL scheme {:?}",
            parsed.scheme()
        );
        return normalize_fallback(fallback);
    }
    let Some(host) = parsed.host_str() else {
        return normalize_fallback(fallback);
    };
    if !ALLOWED_HOSTS.contains(&host) {
        tracing::warn!(
            "nous: refusing inference URL host {host:?} (not in allowlist); using fallback"
        );
        return normalize_fallback(fallback);
    }
    let mut out = parsed.origin().ascii_serialization();
    let path = parsed.path().trim_end_matches('/');
    if !path.is_empty() && path != "/" {
        out.push_str(path);
    }
    out.trim_end_matches('/').to_string()
}

fn normalize_fallback(fallback: &str) -> String {
    let fb = fallback.trim().trim_end_matches('/');
    if fb.ends_with("/v1") {
        fb.to_string()
    } else {
        format!("{fb}/v1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_official_host() {
        let url = validate_nous_inference_url_from_network(
            Some("https://inference-api.nousresearch.com/v1"),
            DEFAULT_NOUS_INFERENCE,
        );
        assert!(url.contains("inference-api.nousresearch.com"));
    }

    #[test]
    fn rejects_evil_host() {
        let url = validate_nous_inference_url_from_network(
            Some("https://evil.example/v1"),
            DEFAULT_NOUS_INFERENCE,
        );
        assert!(url.contains("inference-api.nousresearch.com"));
    }
}
