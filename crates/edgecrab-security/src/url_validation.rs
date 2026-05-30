//! Unified outbound URL validation — SSRF guard + website blocklist.
//!
//! Single entry point for all URL-capable tools (web extract, browser, vision, skills hub).

use thiserror::Error;

use crate::url_safety;
use crate::website_policy;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UrlValidationError {
    #[error("URL blocked by SSRF policy: {0}")]
    SsrfBlocked(String),
    #[error("{0}")]
    WebsitePolicyBlocked(String),
    #[error("URL validation error: {0}")]
    Invalid(String),
}

/// Validate an outbound HTTP(S) URL against SSRF rules and website blocklist.
///
/// Website policy errors fail open on config load failures (logged); explicit
/// `config_path` in [`website_policy::check_website_access`] propagates in tests.
pub fn validate_outbound_url(url: &str) -> Result<(), UrlValidationError> {
    match url_safety::is_safe_url(url) {
        Ok(true) => {}
        Ok(false) => {
            return Err(UrlValidationError::SsrfBlocked(format!(
                "URL blocked by SSRF policy: {url}"
            )));
        }
        Err(e) => {
            return Err(UrlValidationError::Invalid(format!(
                "URL validation error: {e}"
            )));
        }
    }

    match website_policy::check_website_access(url, None) {
        Ok(Some(block)) => Err(UrlValidationError::WebsitePolicyBlocked(block.message)),
        Ok(None) => Ok(()),
        Err(err) => {
            tracing::warn!("Website policy error (failing open): {err}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::website_policy::invalidate_cache;
    use tempfile::TempDir;

    #[test]
    fn blocks_private_ssrf() {
        let err = validate_outbound_url("http://127.0.0.1:8080/secret").expect_err("ssrf");
        assert!(matches!(err, UrlValidationError::SsrfBlocked(_)));
    }

    #[test]
    fn allows_public_url_without_blocklist() {
        invalidate_cache();
        assert!(validate_outbound_url("https://www.rust-lang.org/").is_ok());
    }

    #[test]
    fn blocks_website_policy_domain() {
        invalidate_cache();
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(
            dir.path().join("config.yaml"),
            r#"
security:
  website_blocklist:
    enabled: true
    domains: [blocked.example]
"#,
        )
        .expect("write");
        unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };

        let err = validate_outbound_url("https://docs.blocked.example/page").expect_err("blocked");
        assert!(matches!(err, UrlValidationError::WebsitePolicyBlocked(_)));

        unsafe { std::env::remove_var("EDGECRAB_HOME") };
        invalidate_cache();
    }
}
