//! Nous Portal OAuth refresh (Hermes `resolve_nous_runtime_credentials`).

use std::path::Path;
use std::time::Duration;

use reqwest::Client;

use crate::http_client::build_oauth_http_client;
use serde_json::Value;

use crate::backend::auth_file::{load_auth_doc, write_auth_doc, write_provider_state};
use crate::error::ProxyError;

use super::inference_url::validate_nous_inference_url_from_network;
use super::jwt::{self, INVOKE_JWT_MIN_TTL_SECS, set_agent_key_from_invoke_jwt};
use super::quarantine::{
    NousRefreshFailure, is_terminal_nous_refresh_failure, parse_refresh_failure_body,
    quarantine_nous_pool_in_doc, quarantine_provider_state, state_requires_relogin,
};

pub use super::inference_url::DEFAULT_NOUS_INFERENCE;

pub const DEFAULT_NOUS_PORTAL: &str = "https://portal.nousresearch.com";
pub const DEFAULT_NOUS_CLIENT_ID: &str = "hermes-cli";

fn portal_base_url(state: &Value) -> String {
    state
        .get("portal_base_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_NOUS_PORTAL)
        .trim_end_matches('/')
        .to_string()
}

fn inference_base_url(state: &Value, fallback: &str) -> String {
    validate_nous_inference_url_from_network(
        state
            .get("inference_base_url")
            .or_else(|| state.get("base_url"))
            .and_then(|v| v.as_str()),
        fallback,
    )
}

fn client_id(state: &Value) -> String {
    state
        .get("client_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| DEFAULT_NOUS_CLIENT_ID.to_string())
}

fn build_http_client() -> Result<Client, ProxyError> {
    build_oauth_http_client(Duration::from_secs(15))
}

fn persist_quarantined(
    auth_path: &Path,
    provider: &str,
    state: &Value,
    doc: &mut Value,
) -> Result<(), ProxyError> {
    quarantine_nous_pool_in_doc(doc);
    if let Some(obj) = doc.as_object_mut() {
        let providers = obj
            .entry("providers")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(pmap) = providers.as_object_mut() {
            pmap.insert(provider.to_string(), state.clone());
        }
    }
    write_auth_doc(auth_path, doc)?;
    Ok(())
}

fn handle_refresh_failure(
    auth_path: &Path,
    provider: &str,
    mut state: Value,
    failure: NousRefreshFailure,
) -> Result<(), ProxyError> {
    if is_terminal_nous_refresh_failure(&failure) {
        quarantine_provider_state(&mut state, &failure, "proxy_refresh_failure");
        let mut doc = load_auth_doc(auth_path)?;
        persist_quarantined(auth_path, provider, &state, &mut doc)?;
    }
    Err(ProxyError::UpstreamAuth(format!(
        "nous refresh token exchange failed: {}",
        failure.message
    )))
}

/// POST `{portal}/api/oauth/token` with Hermes headers/body shape.
pub async fn refresh_access_token(
    client: &Client,
    portal_base_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<Value, NousRefreshFailure> {
    let url = format!("{portal_base_url}/api/oauth/token");
    let resp = client
        .post(&url)
        .header("x-nous-refresh-token", refresh_token)
        .form(&[("grant_type", "refresh_token"), ("client_id", client_id)])
        .send()
        .await
        .map_err(|e| NousRefreshFailure {
            code: "network_error".into(),
            message: format!("nous refresh request failed: {e}"),
        })?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| NousRefreshFailure {
        code: "network_error".into(),
        message: format!("nous refresh response body: {e}"),
    })?;
    if status.is_success() {
        let payload: Value = serde_json::from_str(&body).map_err(|e| NousRefreshFailure {
            code: "invalid_json".into(),
            message: format!("nous refresh JSON: {e}"),
        })?;
        if payload
            .get("access_token")
            .and_then(|v| v.as_str())
            .is_some()
        {
            return Ok(payload);
        }
        return Err(NousRefreshFailure {
            code: "invalid_response".into(),
            message: "nous refresh response missing access_token".into(),
        });
    }
    Err(parse_refresh_failure_body(&body, status.as_u16()))
}

async fn resolve_inner(
    auth_path: &Path,
    provider: &str,
    fallback_inference_url: &str,
    force_refresh: bool,
) -> Result<(String, String), ProxyError> {
    let doc = load_auth_doc(auth_path)?;
    let mut state = doc
        .get("providers")
        .and_then(|p| p.get(provider))
        .cloned()
        .ok_or_else(|| {
            ProxyError::UpstreamAuth(format!(
                "provider '{provider}' not found in {}",
                auth_path.display()
            ))
        })?;

    if state_requires_relogin(&state) {
        let msg = state
            .get("last_auth_error")
            .and_then(|e| e.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("relogin required");
        return Err(ProxyError::UpstreamAuth(format!(
            "Nous Portal: {msg} — run `edgecrab auth add nous`"
        )));
    }

    let portal = portal_base_url(&state);
    let mut inference = inference_base_url(&state, fallback_inference_url);
    let cid = client_id(&state);
    let scope = state.get("scope").and_then(|v| v.as_str());

    if !force_refresh
        && let Some(key) = state
            .get("agent_key")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        && jwt::invoke_jwt_is_usable(key, scope, state.get("expires_at").and_then(|v| v.as_str()))
    {
        return Ok((key.to_string(), inference));
    }
    if !force_refresh
        && let Some(access) = state
            .get("access_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        && jwt::invoke_jwt_status(
            access,
            scope,
            state.get("expires_at").and_then(|v| v.as_str()),
            INVOKE_JWT_MIN_TTL_SECS,
        )
        .is_none()
    {
        set_agent_key_from_invoke_jwt(&mut state);
        write_provider_state(auth_path, provider, &state)?;
        let key = state["agent_key"].as_str().expect("agent_key");
        return Ok((key.to_string(), inference));
    }

    let refresh_token = state
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ProxyError::UpstreamAuth(
                "Nous Portal: no refresh token — run `edgecrab auth add nous`".into(),
            )
        })?;

    let client = build_http_client()?;
    let refreshed = match refresh_access_token(&client, &portal, &cid, refresh_token).await {
        Ok(v) => v,
        Err(failure) => {
            handle_refresh_failure(auth_path, provider, state, failure)?;
            unreachable!()
        }
    };

    let access = refreshed["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    state["access_token"] = Value::String(access);
    if let Some(rt) = refreshed.get("refresh_token").and_then(|v| v.as_str()) {
        state["refresh_token"] = Value::String(rt.to_string());
    }
    if let Some(ttl) = refreshed.get("expires_in").and_then(|v| v.as_i64()) {
        state["expires_in"] = Value::from(ttl);
    }
    if let Some(url) = refreshed.get("inference_base_url").and_then(|v| v.as_str()) {
        inference = validate_nous_inference_url_from_network(Some(url), fallback_inference_url);
        state["inference_base_url"] = Value::String(inference.clone());
    }
    set_agent_key_from_invoke_jwt(&mut state);
    write_provider_state(auth_path, provider, &state)?;

    let bearer = state
        .get("agent_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ProxyError::UpstreamAuth("Nous refresh did not yield a usable inference JWT".into())
        })?;
    Ok((bearer.to_string(), inference))
}

/// Resolve inference bearer + base URL; persists rotated tokens to `auth_path` when changed.
pub async fn resolve_nous_credentials_async(
    auth_path: &Path,
    provider: &str,
    fallback_inference_url: &str,
    force_refresh: bool,
) -> Result<(String, String), ProxyError> {
    resolve_inner(auth_path, provider, fallback_inference_url, force_refresh).await
}
