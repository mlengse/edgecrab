//! E2E: Mode A credential forwarder (Hermes-style byte pass-through).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{OriginalUri, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use edgecrab_core::{ForwardUpstreamConfig, ProxyConfig};
use edgecrab_proxy::backend::forwarder::build_forwarder_client;
use edgecrab_proxy::resolve::build_forward_adapters;
use edgecrab_proxy::server::{ProxyState, build_router};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
struct UpstreamCapture {
    auth: Arc<Mutex<Option<String>>>,
    request_uri: Arc<Mutex<Option<String>>>,
}

async fn upstream_chat(
    State(cap): State<UpstreamCapture>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    *cap.request_uri.lock().expect("lock") = Some(uri.to_string());
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    *cap.auth.lock().expect("lock") = auth;
    let _ = body;
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "id": "chatcmpl-upstream",
            "object": "chat.completion",
            "choices": [{"message": {"role": "assistant", "content": "forwarded-ok"}}]
        })),
    )
}

async fn spawn_forward_proxy(
    upstream_base: &str,
    _capture: UpstreamCapture,
) -> (String, tokio::task::JoinHandle<()>) {
    edgecrab_proxy::enable_e2e_direct_http();
    let mut cfg = ProxyConfig::default();
    cfg.model_aliases
        .insert("fwd-model".into(), "forward:mock-up".into());
    cfg.forward_upstreams.insert(
        "mock-up".into(),
        ForwardUpstreamConfig {
            base_url: upstream_base.into(),
            bearer: Some("upstream-secret-bearer".into()),
            ..Default::default()
        },
    );
    let forward_upstreams = cfg.forward_upstreams.clone();
    let state = ProxyState {
        token: "client-proxy-token".into(),
        config: cfg,
        default_model_spec: None,
        forward_adapters: build_forward_adapters(&forward_upstreams),
        forward_client: Arc::new(build_forwarder_client().expect("client")),
        forward_only: None,
    };
    let app = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let base = format!("http://{addr}");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    (base, handle)
}

async fn spawn_forward_only_proxy(upstream_base: &str) -> (String, tokio::task::JoinHandle<()>) {
    let mut cfg = ProxyConfig::default();
    cfg.forward_upstreams.insert(
        "mock-up".into(),
        ForwardUpstreamConfig {
            base_url: upstream_base.into(),
            bearer: Some("upstream-secret-bearer".into()),
            ..Default::default()
        },
    );
    let forward_upstreams = cfg.forward_upstreams.clone();
    let state = ProxyState {
        token: "client-proxy-token".into(),
        config: cfg,
        default_model_spec: None,
        forward_adapters: build_forward_adapters(&forward_upstreams),
        forward_client: Arc::new(build_forwarder_client().expect("client")),
        forward_only: Some("mock-up".into()),
    };
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let base = format!("http://{addr}");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    (base, handle)
}

#[tokio::test]
async fn e2e_forward_mode_replaces_client_auth_and_passes_body() {
    let capture = UpstreamCapture::default();
    let cap_for_upstream = capture.clone();

    let upstream_app = Router::new()
        .route("/v1/chat/completions", post(upstream_chat))
        .with_state(cap_for_upstream);
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let upstream_addr = upstream_listener.local_addr().expect("addr");
    let upstream_base = format!("http://{upstream_addr}/v1");
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_app)
            .await
            .expect("upstream");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (proxy_base, _proxy_handle) = spawn_forward_proxy(&upstream_base, capture.clone()).await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "fwd-model",
        "messages": [{"role": "user", "content": "hi"}]
    });
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer client-proxy-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(json["choices"][0]["message"]["content"], "forwarded-ok");

    let upstream_auth = capture.auth.lock().expect("lock").clone().expect("auth");
    assert_eq!(upstream_auth, "Bearer upstream-secret-bearer");
    assert_ne!(upstream_auth, "Bearer client-proxy-token");
}

#[tokio::test]
async fn e2e_default_forward_upstream_lists_models_via_upstream() {
    let capture = UpstreamCapture::default();
    let cap_for_upstream = capture.clone();

    let _cap = cap_for_upstream;
    let upstream_app = Router::new().route(
        "/v1/models",
        get(|| async {
            axum::Json(serde_json::json!({
                "object": "list",
                "data": [{"id": "upstream-model", "object": "model"}]
            }))
        }),
    );
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let upstream_addr = upstream_listener.local_addr().expect("addr");
    let upstream_base = format!("http://{upstream_addr}/v1");
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_app)
            .await
            .expect("upstream");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut cfg = ProxyConfig {
        default_forward_upstream: Some("mock-up".into()),
        ..Default::default()
    };
    cfg.forward_upstreams.insert(
        "mock-up".into(),
        ForwardUpstreamConfig {
            base_url: upstream_base.clone(),
            bearer: Some("up-bearer".into()),
            ..Default::default()
        },
    );
    let forward_adapters = build_forward_adapters(&cfg.forward_upstreams);
    let state = ProxyState {
        token: "client-proxy-token".into(),
        config: cfg,
        default_model_spec: None,
        forward_adapters,
        forward_client: Arc::new(build_forwarder_client().expect("client")),
        forward_only: None,
    };
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let proxy_addr = listener.local_addr().expect("addr");
    let proxy_base = format!("http://{proxy_addr}");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("proxy");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = edgecrab_proxy::e2e_http_client();
    let resp = client
        .get(format!("{proxy_base}/v1/models"))
        .header("Authorization", "Bearer client-proxy-token")
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(json["data"][0]["id"], "upstream-model");
}

#[tokio::test]
async fn e2e_forward_only_provider_mode_any_model_string() {
    let capture = UpstreamCapture::default();
    let upstream_app = Router::new()
        .route("/v1/chat/completions", post(upstream_chat))
        .with_state(capture.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let upstream_addr = upstream_listener.local_addr().expect("addr");
    let upstream_base = format!("http://{upstream_addr}/v1");
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_app)
            .await
            .expect("upstream");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (proxy_base, _handle) = spawn_forward_only_proxy(&upstream_base).await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "any-model-name-ignored",
        "messages": [{"role": "user", "content": "hi"}]
    });
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer client-proxy-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        capture.auth.lock().expect("lock").as_deref(),
        Some("Bearer upstream-secret-bearer")
    );
}

#[tokio::test]
async fn e2e_forward_preserves_query_string() {
    let capture = UpstreamCapture::default();
    let upstream_app = Router::new()
        .route("/v1/chat/completions", post(upstream_chat))
        .with_state(capture.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let upstream_addr = upstream_listener.local_addr().expect("addr");
    let upstream_base = format!("http://{upstream_addr}/v1");
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_app)
            .await
            .expect("upstream");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (proxy_base, _handle) = spawn_forward_only_proxy(&upstream_base).await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "ignored",
        "messages": [{"role": "user", "content": "hi"}],
        "stream": true
    });
    let resp = client
        .post(format!(
            "{proxy_base}/v1/chat/completions?stream=true&foo=bar"
        ))
        .header("Authorization", "Bearer client-proxy-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let uri = capture
        .request_uri
        .lock()
        .expect("lock")
        .clone()
        .expect("uri");
    assert!(uri.contains("stream=true"), "query not forwarded: {uri}");
    assert!(uri.contains("foo=bar"), "query not forwarded: {uri}");
}

#[tokio::test]
async fn e2e_forward_streaming_sse_passes_through() {
    let capture = UpstreamCapture::default();
    async fn upstream_stream(
        State(cap): State<UpstreamCapture>,
        OriginalUri(uri): OriginalUri,
    ) -> impl IntoResponse {
        *cap.request_uri.lock().expect("lock") = Some(uri.to_string());
        let body = "data: {\"x\":1}\n\ndata: [DONE]\n\n";
        (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
            body,
        )
    }
    let upstream_app = Router::new()
        .route("/v1/chat/completions", post(upstream_stream))
        .with_state(capture.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let upstream_addr = upstream_listener.local_addr().expect("addr");
    let upstream_base = format!("http://{upstream_addr}/v1");
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_app)
            .await
            .expect("upstream");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (proxy_base, _handle) = spawn_forward_only_proxy(&upstream_base).await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "x",
        "messages": [{"role": "user", "content": "hi"}],
        "stream": true
    });
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer client-proxy-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    let text = resp.text().await.expect("body");
    assert!(text.contains("data:"));
    assert!(text.contains("[DONE]"));
}
