//! xAI Grok OAuth PKCE login (`edgecrab auth add xai-oauth` / `edgecrab auth add grok`).

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use url::Url;

use crate::backend::auth_file::{default_auth_path, write_provider_state};
use crate::error::ProxyError;
use crate::http_client::build_oauth_http_client;
use crate::oauth::loopback::{validate_loopback_redirect_uri, LoopbackServer, OAuthCallback, LOOPBACK_HOST};
use edgecrab_core::oauth::pkce::{code_challenge, code_verifier};

use super::refresh::{DEFAULT_XAI_API, XAI_OAUTH_CLIENT_ID, XAI_OAUTH_DISCOVERY_URL};

pub use edgecrab_core::oauth::XAI_OAUTH_PROVIDER;
pub const XAI_OAUTH_SCOPE: &str =
    "openid profile email offline_access grok-cli:access api:access";
pub const XAI_OAUTH_REDIRECT_PORT: u16 = 56121;
pub const XAI_OAUTH_REDIRECT_PATH: &str = "/callback";
pub const XAI_OAUTH_REFERRER: &str = "edgecrab";

#[derive(Debug, Clone)]
pub struct XaiDiscovery {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub raw: Value,
}

/// Prompt payload for CLI/TUI (mirrors Copilot device-code UX hook).
#[derive(Debug, Clone)]
pub struct XaiOAuthAuthorizePrompt {
    pub authorize_url: String,
    pub redirect_uri: String,
    pub manual_paste: bool,
}

#[derive(Clone, Default)]
pub struct XaiOAuthLoginOptions {
    pub open_browser: bool,
    pub manual_paste: bool,
    pub timeout_secs: u64,
    /// When set, EdgeCrab CLI owns sign-in output and browser open (like Copilot `device_code_flow`).
    pub on_authorize: Option<Arc<dyn Fn(XaiOAuthAuthorizePrompt) + Send + Sync>>,
}

impl std::fmt::Debug for XaiOAuthLoginOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XaiOAuthLoginOptions")
            .field("open_browser", &self.open_browser)
            .field("manual_paste", &self.manual_paste)
            .field("timeout_secs", &self.timeout_secs)
            .field("on_authorize", &self.on_authorize.is_some())
            .finish()
    }
}

fn emit_authorize_prompt(opts: &XaiOAuthLoginOptions, authorize_url: &str, redirect_uri: &str) {
    let prompt = XaiOAuthAuthorizePrompt {
        authorize_url: authorize_url.to_string(),
        redirect_uri: redirect_uri.to_string(),
        manual_paste: opts.manual_paste,
    };
    if let Some(hook) = &opts.on_authorize {
        hook(prompt);
        return;
    }
    if opts.manual_paste {
        eprintln!("\nxAI Grok OAuth (manual paste)");
        eprintln!("===========================\n");
    } else {
        eprintln!("\nxAI Grok OAuth");
        eprintln!("==============\n");
    }
    eprintln!("Open this URL to authorize EdgeCrab with xAI:\n{authorize_url}\n");
    if !opts.manual_paste {
        eprintln!("Waiting for callback on {redirect_uri}");
        eprintln!(
            "(Remote host? SSH tunnel: ssh -L {XAI_OAUTH_REDIRECT_PORT}:127.0.0.1:{XAI_OAUTH_REDIRECT_PORT} user@host)\n"
        );
        if opts.open_browser {
            open_browser(authorize_url);
            eprintln!("Browser opened (if supported).");
        }
    }
}

fn build_http_client(timeout: Duration) -> Result<Client, ProxyError> {
    build_oauth_http_client(timeout)
}

fn validate_xai_https_endpoint(url: &str, field: &str) -> Result<String, ProxyError> {
    let parsed = Url::parse(url)
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai {field} parse: {e}")))?;
    if parsed.scheme() != "https" {
        return Err(ProxyError::UpstreamAuth(format!(
            "xAI OIDC discovery returned non-HTTPS {field}"
        )));
    }
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    if host != "x.ai" && !host.ends_with(".x.ai") {
        return Err(ProxyError::UpstreamAuth(format!(
            "xAI OIDC {field} host {host} is not on x.ai"
        )));
    }
    Ok(url.to_string())
}

pub async fn fetch_xai_discovery(client: &Client) -> Result<XaiDiscovery, ProxyError> {
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
    let authorization_endpoint = payload
        .get("authorization_endpoint")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ProxyError::UpstreamAuth("xai discovery missing authorization_endpoint".into())
        })?;
    let token_endpoint = payload
        .get("token_endpoint")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ProxyError::UpstreamAuth("xai discovery missing token_endpoint".into()))?;
    Ok(XaiDiscovery {
        authorization_endpoint: validate_xai_https_endpoint(authorization_endpoint, "authorization_endpoint")?,
        token_endpoint: validate_xai_https_endpoint(token_endpoint, "token_endpoint")?,
        raw: payload,
    })
}

pub fn validate_xai_inference_base_url(value: Option<&str>) -> String {
    let fallback = DEFAULT_XAI_API.trim_end_matches('/').to_string();
    let candidate = value.unwrap_or("").trim().trim_end_matches('/');
    if candidate.is_empty() {
        return fallback;
    }
    let Ok(parsed) = Url::parse(candidate) else {
        return fallback;
    };
    if parsed.scheme() != "https" {
        return fallback;
    }
    let host = parsed.host_str().unwrap_or("").to_lowercase();
    if host != "x.ai" && !host.ends_with(".x.ai") {
        return fallback;
    }
    candidate.to_string()
}

fn build_authorize_url(
    authorization_endpoint: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
    nonce: &str,
) -> String {
    let mut url = Url::parse(authorization_endpoint)
        .unwrap_or_else(|_| Url::parse("https://accounts.x.ai/").expect("fallback"));
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", XAI_OAUTH_CLIENT_ID);
        q.append_pair("redirect_uri", redirect_uri);
        q.append_pair("scope", XAI_OAUTH_SCOPE);
        q.append_pair("code_challenge", code_challenge);
        q.append_pair("code_challenge_method", "S256");
        q.append_pair("state", state);
        q.append_pair("nonce", nonce);
        q.append_pair("plan", "generic");
        q.append_pair("referrer", XAI_OAUTH_REFERRER);
    }
    url.to_string()
}

pub async fn exchange_authorization_code(
    client: &Client,
    token_endpoint: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
    code_challenge: &str,
) -> Result<Value, ProxyError> {
    if code_verifier.is_empty() {
        return Err(ProxyError::UpstreamAuth(
            "xAI token exchange: PKCE code_verifier is empty".into(),
        ));
    }
    let form = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", XAI_OAUTH_CLIENT_ID),
        ("code_verifier", code_verifier),
        ("code_challenge", code_challenge),
        ("code_challenge_method", "S256"),
    ];
    let resp = client
        .post(token_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai token exchange failed: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai token exchange body: {e}")))?;
    if status.as_u16() == 403 {
        return Err(ProxyError::UpstreamAuth(format!(
            "xAI token exchange HTTP 403: {body} — account may lack SuperGrok API access; try XAI_API_KEY or https://x.ai/grok"
        )));
    }
    if !status.is_success() {
        return Err(ProxyError::UpstreamAuth(format!(
            "xAI token exchange HTTP {}: {body}",
            status.as_u16()
        )));
    }
    serde_json::from_str(&body)
        .map_err(|e| ProxyError::UpstreamAuth(format!("xai token exchange JSON: {e}")))
}

fn build_provider_state(
    tokens: Value,
    discovery: &XaiDiscovery,
    redirect_uri: &str,
    base_url: &str,
) -> Value {
    json!({
        "tokens": tokens,
        "discovery": discovery.raw,
        "oauth_discovery": discovery.raw,
        "redirect_uri": redirect_uri,
        "auth_mode": "oauth_pkce",
        "base_url": base_url,
        "last_refresh": Utc::now().to_rfc3339(),
    })
}

fn tokens_from_exchange(payload: &Value) -> Result<Value, ProxyError> {
    let access = payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ProxyError::UpstreamAuth("xAI exchange missing access_token".into()))?;
    let refresh = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ProxyError::UpstreamAuth("xAI exchange missing refresh_token".into()))?;
    Ok(json!({
        "access_token": access,
        "refresh_token": refresh,
        "id_token": payload.get("id_token").and_then(|v| v.as_str()).unwrap_or(""),
        "expires_in": payload.get("expires_in"),
        "token_type": payload.get("token_type").and_then(|v| v.as_str()).unwrap_or("Bearer"),
    }))
}

fn parse_manual_callback_url(input: &str, expected_redirect: &str) -> Result<OAuthCallback, ProxyError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ProxyError::UpstreamAuth("empty callback paste".into()));
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let url = Url::parse(trimmed)
            .map_err(|e| ProxyError::UpstreamAuth(format!("invalid callback URL: {e}")))?;
        let base = Url::parse(expected_redirect)
            .map_err(|e| ProxyError::UpstreamAuth(format!("expected redirect: {e}")))?;
        if url.path() != base.path() {
            return Err(ProxyError::UpstreamAuth("callback URL path mismatch".into()));
        }
        let mut code = None;
        let mut state = None;
        let mut error = None;
        let mut error_description = None;
        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                "error" => error = Some(v.into_owned()),
                "error_description" => error_description = Some(v.into_owned()),
                _ => {}
            }
        }
        return Ok(OAuthCallback {
            code,
            state,
            error,
            error_description,
        });
    }
    Ok(OAuthCallback {
        code: Some(trimmed.to_string()),
        state: None,
        error: None,
        error_description: None,
    })
}

fn prompt_manual_paste(redirect_uri: &str) -> Result<OAuthCallback, ProxyError> {
    eprintln!("\nPaste the full callback URL (or authorization code only):");
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| ProxyError::UpstreamAuth(format!("stdin: {e}")))?;
    parse_manual_callback_url(&line, redirect_uri)
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

/// Run PKCE loopback (or manual-paste) login without persisting.
pub async fn xai_oauth_login(opts: &XaiOAuthLoginOptions) -> Result<Value, ProxyError> {
    let timeout = Duration::from_secs(opts.timeout_secs.max(30));
    let client = build_http_client(Duration::from_secs(20))?;
    let discovery = fetch_xai_discovery(&client).await?;

    let verifier = code_verifier();
    let challenge = code_challenge(&verifier);
    let state = uuid::Uuid::new_v4().simple().to_string();
    let nonce = uuid::Uuid::new_v4().simple().to_string();

    let (redirect_uri, callback) = if opts.manual_paste {
        let redirect_uri = format!(
            "http://{LOOPBACK_HOST}:{XAI_OAUTH_REDIRECT_PORT}{XAI_OAUTH_REDIRECT_PATH}"
        );
        validate_loopback_redirect_uri(&redirect_uri)?;
        let authorize_url = build_authorize_url(
            &discovery.authorization_endpoint,
            &redirect_uri,
            &challenge,
            &state,
            &nonce,
        );
        emit_authorize_prompt(opts, &authorize_url, &redirect_uri);
        let cb = prompt_manual_paste(&redirect_uri)?;
        (redirect_uri, cb)
    } else {
        let server = LoopbackServer::start(XAI_OAUTH_REDIRECT_PORT, XAI_OAUTH_REDIRECT_PATH).await?;
        let redirect_uri = server.redirect_uri.clone();
        validate_loopback_redirect_uri(&redirect_uri)?;
        let authorize_url = build_authorize_url(
            &discovery.authorization_endpoint,
            &redirect_uri,
            &challenge,
            &state,
            &nonce,
        );

        emit_authorize_prompt(opts, &authorize_url, &redirect_uri);

        let wait = server.wait_for_callback(timeout).await;
        let cb = match wait {
            Ok(cb) => cb,
            Err(_) if !opts.manual_paste => {
                eprintln!("Loopback timed out. Paste the callback URL or code:");
                prompt_manual_paste(&redirect_uri)?
            }
            Err(e) => return Err(e),
        };
        server.shutdown().await;
        (redirect_uri, cb)
    };

    if let Some(err) = callback.error.as_deref() {
        let detail = callback
            .error_description
            .as_deref()
            .unwrap_or(err);
        return Err(ProxyError::UpstreamAuth(format!(
            "xAI authorization failed: {detail}"
        )));
    }

    let mut callback_state = callback.state.clone();
    if callback_state.is_none() && opts.manual_paste {
        callback_state = Some(state.clone());
    }
    if callback_state.as_deref() != Some(state.as_str()) {
        return Err(ProxyError::UpstreamAuth("xAI authorization: state mismatch".into()));
    }

    let code = callback
        .code
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ProxyError::UpstreamAuth("xAI authorization: missing code".into()))?;

    let payload = exchange_authorization_code(
        &client,
        &discovery.token_endpoint,
        code,
        &redirect_uri,
        &verifier,
        &challenge,
    )
    .await?;

    let tokens = tokens_from_exchange(&payload)?;
    let base_url = validate_xai_inference_base_url(
        std::env::var("XAI_BASE_URL")
            .ok()
            .as_deref()
            .or(std::env::var("EDGECRAB_XAI_BASE_URL").ok().as_deref()),
    );

    Ok(build_provider_state(tokens, &discovery, &redirect_uri, &base_url))
}

pub async fn persist_xai_oauth(auth_path: &Path, state: Value) -> Result<(), ProxyError> {
    write_provider_state(auth_path, XAI_OAUTH_PROVIDER, &state)?;
    Ok(())
}

/// Full login + persist to `auth_path` (default `~/.edgecrab/auth.json`).
pub async fn login_xai_oauth(
    auth_path: Option<&Path>,
    opts: &XaiOAuthLoginOptions,
) -> Result<String, ProxyError> {
    let path = auth_path
        .map(Path::to_path_buf)
        .unwrap_or_else(default_auth_path);
    let state = xai_oauth_login(opts).await?;
    persist_xai_oauth(&path, state).await?;
    Ok(format!(
        "xAI Grok OAuth saved to {}. Start proxy: edgecrab proxy enable grok && edgecrab proxy start --provider xai",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;
    use axum::Router;
    use tokio::net::TcpListener;

    #[test]
    fn exchange_form_includes_pkce_challenge() {
        let v = code_verifier();
        let c = code_challenge(&v);
        assert!(!c.is_empty());
        assert_ne!(v, c);
    }

    #[tokio::test]
    async fn token_exchange_round_trip_mock_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let token_url = format!("http://{addr}/token");

        let app = Router::new().route(
            "/token",
            post(|form: axum::Form<std::collections::HashMap<String, String>>| async move {
                assert_eq!(form.get("grant_type").map(String::as_str), Some("authorization_code"));
                assert!(form.contains_key("code_verifier"));
                assert_eq!(form.get("code_challenge_method").map(String::as_str), Some("S256"));
                (
                    axum::http::StatusCode::OK,
                    axum::Json(serde_json::json!({
                        "access_token": "at-test",
                        "refresh_token": "rt-test",
                        "expires_in": 3600,
                        "token_type": "Bearer"
                    })),
                )
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        crate::http_client::enable_e2e_direct_http();
        let client = build_http_client(Duration::from_secs(5)).expect("client");
        let verifier = code_verifier();
        let challenge = code_challenge(&verifier);
        let payload = exchange_authorization_code(
            &client,
            &token_url,
            "authcode",
            "http://127.0.0.1:56121/callback",
            &verifier,
            &challenge,
        )
        .await
        .expect("exchange");
        assert_eq!(payload["access_token"], "at-test");
    }

    #[tokio::test]
    async fn full_login_with_simulated_callback() {
        let auth_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let auth_addr = auth_listener.local_addr().expect("addr");
        let discovery = serde_json::json!({
            "authorization_endpoint": format!("http://{auth_addr}/authorize"),
            "token_endpoint": format!("http://{auth_addr}/token"),
        });

        let auth_app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                axum::routing::get({
                    let discovery = discovery.clone();
                    move || async move { axum::Json(discovery.clone()) }
                }),
            )
            .route(
                "/token",
                post(|form: axum::Form<std::collections::HashMap<String, String>>| async move {
                    assert!(form.contains_key("code_verifier"));
                    (
                        axum::http::StatusCode::OK,
                        axum::Json(serde_json::json!({
                            "access_token": "at-mock",
                            "refresh_token": "rt-mock",
                            "expires_in": 3600
                        })),
                    )
                }),
            );
        tokio::spawn(async move {
            axum::serve(auth_listener, auth_app).await.expect("auth");
        });

        crate::http_client::enable_e2e_direct_http();

        let server = LoopbackServer::start(0, XAI_OAUTH_REDIRECT_PATH)
            .await
            .expect("loopback");
        let redirect = server.redirect_uri.clone();
        let verifier = code_verifier();
        let challenge = code_challenge(&verifier);
        let state = "state-test".to_string();
        let nonce = "nonce-test".to_string();

        let client = build_http_client(Duration::from_secs(5)).expect("client");
        let disc = XaiDiscovery {
            authorization_endpoint: format!("http://{auth_addr}/authorize"),
            token_endpoint: format!("http://{auth_addr}/token"),
            raw: discovery,
        };
        let _url = build_authorize_url(&disc.authorization_endpoint, &redirect, &challenge, &state, &nonce);

        let callback_url = format!("{redirect}?code=mockcode&state={state}");
        let http = build_http_client(Duration::from_secs(5)).expect("http");
        let _ = http.get(&callback_url).send().await.expect("hit callback");

        let cb = server
            .wait_for_callback(Duration::from_secs(5))
            .await
            .expect("callback");
        server.shutdown().await;

        assert_eq!(cb.code.as_deref(), Some("mockcode"));
        let payload = exchange_authorization_code(
            &client,
            &disc.token_endpoint,
            "mockcode",
            &redirect,
            &verifier,
            &challenge,
        )
        .await
        .expect("exchange");
        let tokens = tokens_from_exchange(&payload).expect("tokens");
        let built = build_provider_state(tokens, &disc, &redirect, DEFAULT_XAI_API);
        assert_eq!(built["auth_mode"], "oauth_pkce");
    }
}
