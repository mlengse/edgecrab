//! PKCE helpers (RFC 7636) for subscription OAuth providers (024).

use base64::Engine;
use sha2::{Digest, Sha256};

/// Generate a URL-safe PKCE code verifier (43–128 chars).
pub fn code_verifier() -> String {
    let mut raw = Vec::with_capacity(48);
    raw.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    raw.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    raw.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(raw)
        .chars()
        .take(128)
        .collect()
}

/// S256 code challenge for the given verifier.
pub fn code_challenge(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_length_in_range() {
        let v = code_verifier();
        assert!(v.len() >= 43 && v.len() <= 128);
    }

    #[test]
    fn challenge_is_deterministic() {
        let v = "test_verifier_12345678901234567890123456789012";
        assert_eq!(code_challenge(v), code_challenge(v));
        assert_ne!(code_challenge(v), v);
    }
}
