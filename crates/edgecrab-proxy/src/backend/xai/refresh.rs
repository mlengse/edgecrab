//! xAI Grok OAuth refresh (Hermes `refresh_xai_oauth_pure`).

use std::path::Path;
use std::time::Duration;

use reqwest::Client;

use crate::http_client::build_oauth_http_client;
use serde_json::Value;

use crate::backend::auth_file::{bearer_and_base_from_entry, load_auth_doc, write_provider_state};
use crate::backend::auth_store::provider_state_from_doc;
use crate::error::ProxyError;

pub const DEFAULT_XAI_API: &str = "https://api.x.ai/v1";
pub const XAI_OAUTH_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
pub const XAI_OAUTH_DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";

fn build_http_client() -> Result<Client, ProxyError> {
    build_oauth_http_client(Duration::from_secs(20))
}

fn token_endpoint_from_state(state: &Value) -> Option<String> {
    state
        .get("oauth_discovery")
        .or_else(|| state.get("discovery"))
        .and_then(|d| d.get("token_endpoint"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub async fn discover_token_endpoint(client: &Client) -> Result<String, ProxyError> {
    let resp = client
        .get(XAI_OAUTH_DISCOVERY_URL)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai OIDC discovery failed: {e}")))?;
    let body = resp
        .text()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai discovery body: {e}")))?;
    let payload: Value = serde_json::from_str(&body)
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai discovery JSON: {e}")))?;
    let endpoint = payload
        .get("token_endpoint")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ProxyError::UpstreamAuth("xai discovery missing token_endpoint".into()))?;
    Ok(endpoint.to_string())
}

pub async fn refresh_xai_tokens(
    client: &Client,
    token_endpoint: &str,
    refresh_token: &str,
) -> Result<Value, ProxyError> {
    let resp = client
        .post(token_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", XAI_OAUTH_CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai refresh request failed: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai refresh body: {e}")))?;
    if !status.is_success() {
        return Err(ProxyError::UpstreamAuth(format!(
            "xai refresh failed HTTP {status}: {body}"
        )));
    }
    let payload: Value = serde_json::from_str(&body)
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai refresh JSON: {e}")))?;
    if payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .is_some()
    {
        Ok(payload)
    } else {
        Err(ProxyError::UpstreamAuth(
            "xai refresh response missing access_token".into(),
        ))
    }
}

fn refresh_token_from_state(state: &Value) -> Option<String> {
    state
        .get("tokens")
        .and_then(|t| t.get("refresh_token"))
        .or_else(|| state.get("refresh_token"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Resolve bearer for xAI; refreshes OAuth tokens in `providers.<provider>` when needed.
pub async fn resolve_xai_credentials_async(
    auth_path: &Path,
    provider: &str,
    fallback_base: &str,
    pool_index: usize,
    force_refresh: bool,
) -> Result<(String, String), ProxyError> {
    let doc = load_auth_doc(auth_path)?;
    let pool = crate::backend::auth_file::read_credential_pool_entries(&doc, provider);
    if !force_refresh
        && let Some(entry) = pool.get(pool_index)
        && let Some(pair) = bearer_and_base_from_entry(entry, fallback_base)
    {
        return Ok(pair);
    }

    let mut state = provider_state_from_doc(&doc, provider).ok_or_else(|| {
        ProxyError::UpstreamAuth(format!(
            "provider '{provider}' not found in {}",
            auth_path.display()
        ))
    })?;

    if !force_refresh && let Some(pair) = bearer_and_base_from_entry(&state, fallback_base) {
        return Ok(pair);
    }

    let refresh_token = refresh_token_from_state(&state).ok_or_else(|| {
        ProxyError::UpstreamAuth(
            "xAI OAuth: no refresh_token — run `edgecrab auth add xai-oauth` or `edgecrab auth add grok`".into(),
        )
    })?;

    let client = build_http_client()?;
    let endpoint = if let Some(ep) = token_endpoint_from_state(&state) {
        ep
    } else {
        discover_token_endpoint(&client).await?
    };
    let refreshed = refresh_xai_tokens(&client, &endpoint, &refresh_token).await?;
    let access = refreshed["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    let tokens = state
        .as_object_mut()
        .and_then(|o| {
            if !o.contains_key("tokens") {
                o.insert("tokens".into(), Value::Object(serde_json::Map::new()));
            }
            o.get_mut("tokens")
        })
        .and_then(|t| t.as_object_mut())
        .ok_or_else(|| {
            ProxyError::UpstreamAuth("xai provider state missing tokens object".into())
        })?;
    tokens.insert("access_token".into(), Value::String(access.clone()));
    if let Some(rt) = refreshed.get("refresh_token").and_then(|v| v.as_str()) {
        tokens.insert("refresh_token".into(), Value::String(rt.to_string()));
    }
    if let Some(ttl) = refreshed.get("expires_in") {
        tokens.insert("expires_in".into(), ttl.clone());
    }
    write_provider_state(auth_path, provider, &state)?;
    let base = state
        .get("base_url")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_base.trim_end_matches('/').to_string());
    Ok((access, base))
}
