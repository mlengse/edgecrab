//! Nous Portal device-code OAuth (`edgecrab auth add nous`).

use std::path::Path;
use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::backend::auth_file::{default_auth_path, write_provider_state};
use crate::error::ProxyError;
use crate::http_client::build_oauth_http_client;

use super::inference_url::validate_nous_inference_url_from_network;
use super::jwt::INFERENCE_INVOKE_SCOPE;
use super::refresh::{DEFAULT_NOUS_INFERENCE, DEFAULT_NOUS_PORTAL, resolve_nous_credentials_async};

pub const DEFAULT_NOUS_CLIENT_ID: &str = "hermes-cli";
const POLL_INTERVAL_CAP_SECS: u64 = 1;

#[derive(Debug, Clone)]
pub struct NousDeviceLoginOptions {
    pub portal_base_url: Option<String>,
    pub inference_base_url: Option<String>,
    pub client_id: Option<String>,
    pub scope: Option<String>,
    pub open_browser: bool,
    pub timeout_secs: u64,
}

impl Default for NousDeviceLoginOptions {
    fn default() -> Self {
        Self {
            portal_base_url: None,
            inference_base_url: None,
            client_id: None,
            scope: None,
            open_browser: true,
            timeout_secs: 15,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri_complete: String,
    expires_in: u64,
    interval: u64,
}

fn portal_url(opts: &NousDeviceLoginOptions) -> String {
    opts.portal_base_url
        .as_deref()
        .unwrap_or(DEFAULT_NOUS_PORTAL)
        .trim_end_matches('/')
        .to_string()
}

fn inference_url(opts: &NousDeviceLoginOptions) -> String {
    validate_nous_inference_url_from_network(
        opts.inference_base_url.as_deref(),
        DEFAULT_NOUS_INFERENCE,
    )
}

fn client_id(opts: &NousDeviceLoginOptions) -> String {
    opts.client_id
        .clone()
        .unwrap_or_else(|| DEFAULT_NOUS_CLIENT_ID.to_string())
}

fn scope(opts: &NousDeviceLoginOptions) -> String {
    opts.scope
        .clone()
        .unwrap_or_else(|| INFERENCE_INVOKE_SCOPE.to_string())
}

async fn request_device_code(
    client: &Client,
    portal: &str,
    client_id: &str,
    scope: &str,
) -> Result<DeviceCodeResponse, ProxyError> {
    let url = format!("{portal}/api/oauth/device/code");
    let resp = client
        .post(&url)
        .form(&[("client_id", client_id), ("scope", scope)])
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("nous device code request failed: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("nous device code body: {e}")))?;
    if !status.is_success() {
        return Err(ProxyError::UpstreamAuth(format!(
            "nous device code HTTP {}: {body}",
            status.as_u16()
        )));
    }
    serde_json::from_str(&body)
        .map_err(|e| ProxyError::UpstreamAuth(format!("nous device code JSON: {e}")))
}

async fn poll_device_token(
    client: &Client,
    portal: &str,
    client_id: &str,
    device_code: &str,
    expires_in: u64,
    poll_interval: u64,
) -> Result<Value, ProxyError> {
    let url = format!("{portal}/api/oauth/token");
    let deadline = Instant::now() + Duration::from_secs(expires_in.max(1));
    let mut interval_secs = poll_interval.clamp(1, POLL_INTERVAL_CAP_SECS);

    while Instant::now() < deadline {
        let resp = client
            .post(&url)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", device_code),
            ])
            .send()
            .await
            .map_err(|e| ProxyError::UpstreamAuth(format!("nous token poll failed: {e}")))?;

        if resp.status().is_success() {
            let payload: Value = resp
                .json()
                .await
                .map_err(|e| ProxyError::UpstreamAuth(format!("nous token JSON: {e}")))?;
            if payload
                .get("access_token")
                .and_then(|v| v.as_str())
                .is_some()
            {
                return Ok(payload);
            }
            return Err(ProxyError::UpstreamAuth(
                "nous token response missing access_token".into(),
            ));
        }

        let body = resp.text().await.unwrap_or_default();
        let error_code = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
            .unwrap_or_else(|| "unknown".into());

        match error_code.as_str() {
            "authorization_pending" => {
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }
            "slow_down" => {
                interval_secs = (interval_secs + 1).min(30);
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }
            other => {
                let desc = serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|v| {
                        v.get("error_description")
                            .and_then(|d| d.as_str())
                            .map(str::to_string)
                    })
                    .unwrap_or(body);
                return Err(ProxyError::UpstreamAuth(format!("{other}: {desc}")));
            }
        }
    }

    Err(ProxyError::UpstreamAuth(
        "timed out waiting for Nous device authorization".into(),
    ))
}

fn build_auth_state(
    portal: &str,
    inference: &str,
    client_id: &str,
    scope: &str,
    token: &Value,
) -> Value {
    let now = Utc::now();
    let expires_in = token
        .get("expires_in")
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n as u64)))
        .unwrap_or(0);
    let expires_at = now + chrono::Duration::seconds(expires_in as i64);
    let resolved_inference = token
        .get("inference_base_url")
        .and_then(|v| v.as_str())
        .map(|u| validate_nous_inference_url_from_network(Some(u), inference))
        .unwrap_or_else(|| inference.to_string());

    json!({
        "portal_base_url": portal,
        "inference_base_url": resolved_inference,
        "client_id": client_id,
        "scope": token.get("scope").and_then(|v| v.as_str()).unwrap_or(scope),
        "token_type": token.get("token_type").and_then(|v| v.as_str()).unwrap_or("Bearer"),
        "access_token": token.get("access_token").and_then(|v| v.as_str()),
        "refresh_token": token.get("refresh_token").and_then(|v| v.as_str()),
        "obtained_at": now.to_rfc3339(),
        "expires_at": expires_at.to_rfc3339(),
        "expires_in": expires_in,
        "agent_key": null,
        "agent_key_id": null,
        "agent_key_expires_at": null,
    })
}

/// Run the Nous Portal device-code flow (does not persist).
pub async fn nous_device_code_login(opts: &NousDeviceLoginOptions) -> Result<Value, ProxyError> {
    let portal = portal_url(opts);
    let inference = inference_url(opts);
    let client_id = client_id(opts);
    let scope = scope(opts);
    let timeout = Duration::from_secs(opts.timeout_secs.max(5));
    let client = build_oauth_http_client(timeout)?;

    let device = request_device_code(&client, &portal, &client_id, &scope).await?;

    eprintln!("\nNous Portal sign-in");
    eprintln!("====================\n");
    eprintln!("To continue:");
    eprintln!("  1. Open: {}", device.verification_uri_complete);
    eprintln!("  2. If prompted, enter code: {}\n", device.user_code);

    if opts.open_browser {
        open_browser(&device.verification_uri_complete);
    }

    eprintln!(
        "Waiting for approval (polling every {}s)...",
        device.interval.clamp(1, POLL_INTERVAL_CAP_SECS)
    );

    let token = poll_device_token(
        &client,
        &portal,
        &client_id,
        &device.device_code,
        device.expires_in,
        device.interval,
    )
    .await?;

    let mut state = build_auth_state(&portal, &inference, &client_id, &scope, &token);
    finalize_nous_state(&mut state, &portal, &client_id, &inference).await?;
    Ok(state)
}

async fn finalize_nous_state(
    state: &mut Value,
    portal: &str,
    client_id: &str,
    inference: &str,
) -> Result<(), ProxyError> {
    let scope = state.get("scope").and_then(|v| v.as_str());
    if let Some(access) = state.get("access_token").and_then(|v| v.as_str())
        && super::jwt::invoke_jwt_is_usable(
            access,
            scope,
            state.get("expires_at").and_then(|v| v.as_str()),
        )
    {
        super::jwt::set_agent_key_from_invoke_jwt(state);
        if state
            .get("agent_key")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
        {
            return Ok(());
        }
    }

    let refresh_token = state
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ProxyError::UpstreamAuth("nous login missing refresh_token".into()))?;

    let client = build_oauth_http_client(Duration::from_secs(15))?;
    let url = format!("{portal}/api/oauth/token");
    let resp = client
        .post(&url)
        .header("x-nous-refresh-token", refresh_token)
        .form(&[("grant_type", "refresh_token"), ("client_id", client_id)])
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("nous finalize refresh: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("nous finalize body: {e}")))?;
    if !status.is_success() {
        return Err(ProxyError::UpstreamAuth(format!(
            "nous finalize HTTP {}: {body}",
            status.as_u16()
        )));
    }
    let refreshed: Value = serde_json::from_str(&body)
        .map_err(|e| ProxyError::UpstreamAuth(format!("nous finalize JSON: {e}")))?;
    if let Some(access) = refreshed.get("access_token").and_then(|v| v.as_str()) {
        state["access_token"] = Value::String(access.to_string());
    }
    if let Some(rt) = refreshed.get("refresh_token").and_then(|v| v.as_str()) {
        state["refresh_token"] = Value::String(rt.to_string());
    }
    if let Some(url) = refreshed.get("inference_base_url").and_then(|v| v.as_str()) {
        state["inference_base_url"] = Value::String(validate_nous_inference_url_from_network(
            Some(url),
            inference,
        ));
    }
    super::jwt::set_agent_key_from_invoke_jwt(state);
    if state
        .get("agent_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .is_none()
    {
        return Err(ProxyError::UpstreamAuth(
            "nous login did not yield a usable inference JWT".into(),
        ));
    }
    Ok(())
}

/// Persist Hermes-shaped `providers.nous` and mint invoke JWT (proxy-ready).
pub async fn persist_nous_oauth(
    auth_path: &Path,
    mut state: Value,
    label: Option<&str>,
) -> Result<(), ProxyError> {
    if let Some(label) = label.filter(|s| !s.is_empty())
        && let Some(obj) = state.as_object_mut()
    {
        obj.insert("label".into(), Value::String(label.to_string()));
    }
    let inference = state
        .get("inference_base_url")
        .and_then(|v| v.as_str())
        .map(|s| validate_nous_inference_url_from_network(Some(s), DEFAULT_NOUS_INFERENCE))
        .unwrap_or_else(|| DEFAULT_NOUS_INFERENCE.to_string());
    let portal = state
        .get("portal_base_url")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_NOUS_PORTAL)
        .trim_end_matches('/')
        .to_string();
    let client_id = state
        .get("client_id")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_NOUS_CLIENT_ID)
        .to_string();
    if state
        .get("agent_key")
        .and_then(|v| v.as_str())
        .is_none_or(|s| s.is_empty())
    {
        finalize_nous_state(&mut state, &portal, &client_id, &inference).await?;
    }
    write_provider_state(auth_path, "nous", &state)?;
    let _ = resolve_nous_credentials_async(auth_path, "nous", &inference, false).await?;
    Ok(())
}

/// Full login: device flow + persist to `auth_path` (default `~/.edgecrab/auth.json`).
pub async fn login_nous_portal(
    auth_path: Option<&Path>,
    opts: &NousDeviceLoginOptions,
    label: Option<&str>,
) -> Result<String, ProxyError> {
    let path = auth_path
        .map(Path::to_path_buf)
        .unwrap_or_else(default_auth_path);
    let state = nous_device_code_login(opts).await?;
    persist_nous_oauth(&path, state, label).await?;
    Ok(format!(
        "Nous Portal OAuth saved to {}. Start proxy: edgecrab proxy start --provider nous",
        path.display()
    ))
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).status();

    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).status();

    #[cfg(windows)]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::post;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn device_flow_round_trip_mock_portal() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let portal = format!("http://{addr}");
        let approved = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let approved_poll = approved.clone();
        let app = Router::new()
            .route(
                "/api/oauth/device/code",
                post(|| async {
                    axum::Json(serde_json::json!({
                        "device_code": "dc-test",
                        "user_code": "ABCD-1234",
                        "verification_uri": "https://portal.example/device",
                        "verification_uri_complete": "https://portal.example/device?code=ABCD-1234",
                        "expires_in": 60,
                        "interval": 1
                    }))
                }),
            )
            .route(
                "/api/oauth/token",
                post(
                    move |form: axum::Form<std::collections::HashMap<String, String>>| {
                        let approved_poll = approved_poll.clone();
                        async move {
                            if form.get("grant_type").map(String::as_str)
                                == Some("urn:ietf:params:oauth:grant-type:device_code")
                                && !approved_poll.swap(true, std::sync::atomic::Ordering::SeqCst)
                            {
                                return (
                                    axum::http::StatusCode::BAD_REQUEST,
                                    axum::Json(serde_json::json!({
                                        "error": "authorization_pending"
                                    })),
                                );
                            }
                            (
                                axum::http::StatusCode::OK,
                                axum::Json(serde_json::json!({
                                    "access_token": make_test_invoke_jwt(),
                                    "refresh_token": "rt-test",
                                    "expires_in": 7200,
                                    "token_type": "Bearer",
                                    "scope": INFERENCE_INVOKE_SCOPE
                                })),
                            )
                        }
                    },
                ),
            );
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        tokio::time::sleep(Duration::from_millis(80)).await;

        crate::http_client::enable_e2e_direct_http();
        let state = nous_device_code_login(&NousDeviceLoginOptions {
            portal_base_url: Some(portal),
            inference_base_url: Some("http://127.0.0.1:9/v1".into()),
            open_browser: false,
            timeout_secs: 10,
            ..Default::default()
        })
        .await
        .expect("login");

        assert!(state.get("refresh_token").is_some());
        assert!(state.get("access_token").is_some());
    }

    fn make_test_invoke_jwt() -> String {
        use base64::Engine;
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"alg":"none","typ":"JWT"}"#);
        let exp = (Utc::now().timestamp() + 7200).to_string();
        let payload = serde_json::json!({
            "scope": INFERENCE_INVOKE_SCOPE,
            "exp": exp.parse::<i64>().expect("exp timestamp")
        });
        let body = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).expect("jwt payload"));
        format!("{header}.{body}.sig")
    }
}
