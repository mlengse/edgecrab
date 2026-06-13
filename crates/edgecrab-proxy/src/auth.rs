//! Local bearer token for inbound proxy clients.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, bail};
use subtle::ConstantTimeEq;

use crate::error::ProxyError;

/// Load the expected proxy token from disk (trimmed, no trailing newline).
pub fn load_proxy_token(path: &Path) -> anyhow::Result<String> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read proxy token at {}", path.display()))?;
    let token = raw.trim().to_string();
    if token.is_empty() {
        bail!("proxy token file is empty: {}", path.display());
    }
    Ok(token)
}

/// Create or rotate the proxy token file (`chmod 0600` on Unix).
pub fn write_proxy_token(path: &Path, token: Option<&str>) -> anyhow::Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let value = match token {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => uuid::Uuid::new_v4().to_string(),
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("open proxy token path {}", path.display()))?;
    file.write_all(value.as_bytes())?;
    file.write_all(b"\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(value)
}

/// Ensure a token exists; generate one when missing.
pub fn ensure_proxy_token(path: &Path) -> anyhow::Result<String> {
    if path.exists() {
        return load_proxy_token(path);
    }
    write_proxy_token(path, None)
}

/// Timing-safe bearer check (`Authorization: Bearer <token>`).
pub fn check_bearer(headers: &axum::http::HeaderMap, expected: &str) -> Result<(), ProxyError> {
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let provided = auth.strip_prefix("Bearer ").unwrap_or(auth).trim();
    if provided.is_empty() {
        return Err(ProxyError::Unauthorized(
            "Missing Authorization header (expected Bearer token)".into(),
        ));
    }
    if !bool::from(provided.as_bytes().ct_eq(expected.as_bytes())) {
        return Err(ProxyError::Unauthorized(
            "Incorrect API key provided".into(),
        ));
    }
    Ok(())
}

/// Refuse non-localhost bind without a configured token (mirrors gateway API server).
pub fn validate_bind_address(host: &str, token_path: &Path) -> anyhow::Result<()> {
    let is_local = host == "127.0.0.1" || host == "::1" || host.eq_ignore_ascii_case("localhost");
    if !is_local && !token_path.exists() {
        bail!(
            "proxy token required when binding to non-localhost address '{host}'. \
             Run `edgecrab proxy token set` or bind to 127.0.0.1."
        );
    }
    Ok(())
}

/// Public bind requires explicit opt-in plus a non-empty token file.
pub fn validate_public_bind(
    allow_public: bool,
    host: &str,
    token_path: &Path,
) -> anyhow::Result<()> {
    let is_local = host == "127.0.0.1" || host == "::1" || host.eq_ignore_ascii_case("localhost");
    if is_local {
        return Ok(());
    }
    if !allow_public {
        bail!(
            "binding to '{host}' requires `--allow-public` (and a proxy token). \
             Default is loopback-only."
        );
    }
    if !token_path.exists() {
        bail!(
            "`--allow-public` requires a proxy token at {}",
            token_path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::path::PathBuf;

    #[test]
    fn bearer_rejects_malformed_and_wrong_key() {
        let mut headers = axum::http::HeaderMap::new();
        assert!(check_bearer(&headers, "secret").is_err());

        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer wrong"),
        );
        assert!(check_bearer(&headers, "secret").is_err());

        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret"),
        );
        assert!(check_bearer(&headers, "secret").is_ok());
    }

    #[test]
    fn public_bind_requires_allow_public_flag() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("token");
        write_proxy_token(&path, Some("tok")).expect("write");
        assert!(validate_public_bind(false, "0.0.0.0", &path).is_err());
        assert!(validate_public_bind(true, "0.0.0.0", &path).is_ok());
    }

    #[test]
    fn non_local_bind_requires_token_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("missing-token");
        assert!(validate_bind_address("192.168.1.5", &path).is_err());
    }

    #[test]
    fn token_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path: PathBuf = dir.path().join("proxy-token");
        let t = write_proxy_token(&path, Some("test-token-abc")).expect("write");
        assert_eq!(t, "test-token-abc");
        assert_eq!(load_proxy_token(&path).expect("load"), "test-token-abc");
    }
}
