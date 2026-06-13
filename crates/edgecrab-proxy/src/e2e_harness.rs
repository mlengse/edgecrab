//! Shared mock upstream + OAuth harness for integration tests (grok / xAI).

#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{OriginalUri, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use edgecrab_core::{ForwardAdapterKind, ProxyConfig};

use crate::backend::forwarder::build_forwarder_client;
use crate::guide::{RECIPE_XAI, apply_recipe};
use crate::http_client::enable_e2e_direct_http;
use crate::resolve::build_forward_adapters;
use crate::server::{ProxyState, build_router};
use reqwest::Client;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// HTTP client for calling the local proxy in tests (never use system proxy).
pub fn e2e_http_client() -> Client {
    Client::builder()
        .no_proxy()
        .build()
        .expect("e2e HTTP client")
}

/// Captures upstream Authorization and optional request URI (query forwarding).
#[derive(Clone, Default)]
pub struct UpstreamCapture {
    pub auth: Arc<Mutex<Option<String>>>,
    pub request_uri: Arc<Mutex<Option<String>>>,
    pub refresh_hits: Arc<Mutex<u32>>,
}

pub async fn upstream_chat(
    State(cap): State<UpstreamCapture>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    *cap.request_uri.lock().expect("lock") = Some(uri.to_string());
    *cap.auth.lock().expect("lock") = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let _ = body;
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "id": "chatcmpl-grok",
            "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "grok"}}]
        })),
    )
}

pub async fn upstream_models() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "object": "list",
            "data": [{"id": "grok-4", "object": "model"}]
        })),
    )
}

pub async fn oauth_refresh(State(cap): State<UpstreamCapture>) -> impl IntoResponse {
    *cap.refresh_hits.lock().expect("lock") += 1;
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "access_token": "fresh-xai-access",
            "refresh_token": "rt-rotated",
            "expires_in": 3600,
            "token_type": "Bearer"
        })),
    )
}

pub async fn upstream_stream(
    State(cap): State<UpstreamCapture>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> impl IntoResponse {
    *cap.request_uri.lock().expect("lock") = Some(uri.to_string());
    *cap.auth.lock().expect("lock") = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n";
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
        body,
    )
}

/// Mock OAuth token endpoint + chat upstream; returns `(token_url, upstream_base_v1, capture)`.
pub async fn spawn_xai_mock_stack() -> (String, String, UpstreamCapture) {
    enable_e2e_direct_http();
    let capture = UpstreamCapture::default();

    let oauth_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind oauth");
    let oauth_addr = oauth_listener.local_addr().expect("addr");
    let token_url = format!("http://{oauth_addr}/oauth/token");
    let oauth_app = Router::new()
        .route("/oauth/token", post(oauth_refresh))
        .with_state(capture.clone());
    tokio::spawn(async move {
        axum::serve(oauth_listener, oauth_app)
            .await
            .expect("oauth serve");
    });

    let upstream_app = Router::new()
        .route("/v1/chat/completions", post(upstream_chat))
        .route("/v1/models", get(upstream_models))
        .with_state(capture.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind upstream");
    let upstream_addr = upstream_listener.local_addr().expect("addr");
    let upstream_base = format!("http://{upstream_addr}/v1");
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_app)
            .await
            .expect("upstream serve");
    });

    tokio::time::sleep(Duration::from_millis(80)).await;
    (token_url, upstream_base, capture)
}

pub fn write_xai_auth_json(path: &std::path::Path, token_url: &str) {
    write_xai_auth_json_with_access(path, token_url, None);
}

/// `access_token` = `Some` skips OAuth refresh (fast path for list-models e2e).
pub fn write_xai_auth_json_with_access(
    path: &std::path::Path,
    token_url: &str,
    access_token: Option<&str>,
) {
    let mut tokens = serde_json::json!({ "refresh_token": "rt-xai-test" });
    if let Some(tok) = access_token {
        tokens["access_token"] = serde_json::Value::String(tok.into());
    }
    std::fs::write(
        path,
        serde_json::json!({
            "providers": {
                "xai-oauth": {
                    "tokens": tokens,
                    "oauth_discovery": { "token_endpoint": token_url }
                }
            }
        })
        .to_string(),
    )
    .expect("write auth.json");
}

/// Config after `edgecrab proxy setup grok` (recipe + mock upstream base for tests).
pub fn grok_recipe_proxy_config(upstream_base: &str, auth_path: std::path::PathBuf) -> ProxyConfig {
    let mut cfg = ProxyConfig::default();
    apply_recipe(&mut cfg, &RECIPE_XAI);
    if let Some(up) = cfg.forward_upstreams.get_mut("xai") {
        up.base_url = upstream_base.into();
        up.auth_path = Some(auth_path);
        up.adapter = ForwardAdapterKind::XaiOauth;
        up.auth_provider = Some("xai-oauth".into());
    }
    cfg
}

pub async fn spawn_proxy(
    cfg: ProxyConfig,
    forward_only: Option<String>,
    client_token: &str,
) -> (String, JoinHandle<()>) {
    enable_e2e_direct_http();
    let forward_upstreams = cfg.forward_upstreams.clone();
    let state = ProxyState {
        token: client_token.into(),
        config: cfg,
        default_model_spec: None,
        forward_adapters: build_forward_adapters(&forward_upstreams),
        forward_client: Arc::new(build_forwarder_client().expect("forward client")),
        forward_only,
    };
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind proxy");
    let addr = listener.local_addr().expect("addr");
    let base = format!("http://{addr}");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("proxy serve");
    });
    tokio::time::sleep(Duration::from_millis(80)).await;
    (base, handle)
}
