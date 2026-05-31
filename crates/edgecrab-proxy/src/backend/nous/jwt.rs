//! Nous invoke JWT checks (Hermes `_nous_invoke_jwt_status` / `_set_nous_agent_key_from_invoke_jwt`).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::Value;

pub const INFERENCE_INVOKE_SCOPE: &str = "inference:invoke";
/// Refresh when fewer than this many seconds remain (Hermes `ACCESS_TOKEN_REFRESH_SKEW_SECONDS`).
pub const INVOKE_JWT_MIN_TTL_SECS: i64 = 120;

/// Decode JWT payload without signature verification (same as Hermes runtime checks).
pub fn decode_jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload.as_bytes()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn scope_values(scope: &Value) -> Vec<String> {
    match scope {
        Value::String(s) => s.split_whitespace().map(str::to_string).collect(),
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    }
}

fn token_scopes(claims: &Value, state_scope: Option<&str>) -> Vec<String> {
    let mut scopes = scope_values(claims.get("scope").unwrap_or(&Value::Null));
    scopes.extend(scope_values(claims.get("scp").unwrap_or(&Value::Null)));
    if let Some(s) = state_scope {
        scopes.extend(scope_values(&Value::String(s.to_string())));
    }
    scopes.sort();
    scopes.dedup();
    scopes
}

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `None` when the token can be used for inference; otherwise a reason string.
pub fn invoke_jwt_status(
    token: &str,
    state_scope: Option<&str>,
    _expires_at: Option<&str>,
    min_ttl_secs: i64,
) -> Option<String> {
    let Some(claims) = decode_jwt_claims(token) else {
        return Some("access_token_not_jwt".into());
    };
    let scopes = token_scopes(&claims, state_scope);
    if !scopes.iter().any(|s| s == INFERENCE_INVOKE_SCOPE) {
        return Some("missing_inference_invoke_scope".into());
    }
    let skew = min_ttl_secs.max(0);
    let exp = claims.get("exp").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))?;
    if exp <= now_epoch() + skew {
        return Some("invoke_jwt_expiring".into());
    }
    None
}

pub fn invoke_jwt_is_usable(
    token: &str,
    state_scope: Option<&str>,
    expires_at: Option<&str>,
) -> bool {
    invoke_jwt_status(token, state_scope, expires_at, INVOKE_JWT_MIN_TTL_SECS).is_none()
}

/// Mirror Hermes `_set_nous_agent_key_from_invoke_jwt`.
pub fn set_agent_key_from_invoke_jwt(state: &mut Value) {
    let access = state
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let Some(access) = access else {
        return;
    };
    if let Some(exp) = decode_jwt_claims(&access).and_then(|c| c.get("exp").and_then(|v| v.as_i64())) {
        let ttl = (exp - now_epoch()).max(0);
        state["expires_in"] = Value::from(ttl);
    }
    state["agent_key"] = Value::String(access);
}

/// Build an unsigned JWT for tests (invoke scope + `exp`).
pub fn make_jwt(exp: i64, scope: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let payload = serde_json::json!({"exp": exp, "scope": scope});
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
    format!("{header}.{payload_b64}.sig")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_invoke_jwt_has_scope_and_future_exp() {
        let exp = now_epoch() + 3600;
        let jwt = make_jwt(exp, INFERENCE_INVOKE_SCOPE);
        assert!(invoke_jwt_is_usable(&jwt, None, None));
    }

    #[test]
    fn expiring_jwt_returns_reason() {
        let jwt = make_jwt(1, INFERENCE_INVOKE_SCOPE);
        assert_eq!(
            invoke_jwt_status(&jwt, None, None, INVOKE_JWT_MIN_TTL_SECS),
            Some("invoke_jwt_expiring".into())
        );
    }
}
