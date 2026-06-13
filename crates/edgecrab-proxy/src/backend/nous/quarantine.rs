//! Terminal OAuth failure handling (Hermes `_quarantine_nous_*`).

use chrono::Utc;
use serde_json::{Map, Value};

const TERMINAL_CODES: &[&str] = &["invalid_grant", "invalid_token", "refresh_token_reused"];

const STRIP_KEYS: &[&str] = &[
    "access_token",
    "refresh_token",
    "expires_at",
    "expires_in",
    "obtained_at",
    "agent_key",
    "agent_key_id",
    "agent_key_expires_at",
    "agent_key_expires_in",
    "agent_key_reused",
    "agent_key_obtained_at",
];

/// Parsed OAuth error from a failed refresh response.
pub struct NousRefreshFailure {
    pub code: String,
    pub message: String,
}

pub fn parse_refresh_failure_body(body: &str, http_status: u16) -> NousRefreshFailure {
    if let Ok(j) = serde_json::from_str::<Value>(body) {
        let code = j
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("oauth_error")
            .to_string();
        let desc = j
            .get("error_description")
            .and_then(|v| v.as_str())
            .unwrap_or(body);
        return NousRefreshFailure {
            code: code.clone(),
            message: format!("{code}: {desc}"),
        };
    }
    NousRefreshFailure {
        code: format!("http_{http_status}"),
        message: body.to_string(),
    }
}

pub fn is_terminal_nous_refresh_failure(failure: &NousRefreshFailure) -> bool {
    TERMINAL_CODES
        .iter()
        .any(|c| failure.code.eq_ignore_ascii_case(c))
}

/// Strip dead OAuth material and record `last_auth_error` (Hermes `_quarantine_nous_oauth_state`).
pub fn quarantine_provider_state(state: &mut Value, failure: &NousRefreshFailure, reason: &str) {
    let Some(obj) = state.as_object_mut() else {
        return;
    };
    for key in STRIP_KEYS {
        obj.remove(*key);
    }
    obj.insert(
        "last_auth_error".into(),
        Value::Object(Map::from_iter([
            ("provider".into(), Value::String("nous".into())),
            ("code".into(), Value::String(failure.code.clone())),
            ("message".into(), Value::String(failure.message.clone())),
            ("reason".into(), Value::String(reason.into())),
            ("relogin_required".into(), Value::Bool(true)),
            ("at".into(), Value::String(Utc::now().to_rfc3339())),
        ])),
    );
}

/// Remove `credential_pool.nous` entries that still carry refresh tokens (Hermes pool quarantine).
pub fn quarantine_nous_pool_in_doc(doc: &mut Value) {
    let Some(pool) = doc.get_mut("credential_pool") else {
        return;
    };
    let Some(obj) = pool.as_object_mut() else {
        return;
    };
    obj.remove("nous");
}

pub fn state_requires_relogin(state: &Value) -> bool {
    state
        .get("last_auth_error")
        .and_then(|e| e.get("relogin_required"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_invalid_grant() {
        let f = parse_refresh_failure_body(
            r#"{"error":"invalid_grant","error_description":"revoked"}"#,
            400,
        );
        assert!(is_terminal_nous_refresh_failure(&f));
    }

    #[test]
    fn quarantine_strips_tokens() {
        let mut state = serde_json::json!({
            "refresh_token": "rt",
            "agent_key": "jwt"
        });
        let f = NousRefreshFailure {
            code: "invalid_grant".into(),
            message: "invalid_grant: revoked".into(),
        };
        quarantine_provider_state(&mut state, &f, "proxy_refresh_failure");
        assert!(state.get("refresh_token").is_none());
        assert!(state_requires_relogin(&state));
    }
}
