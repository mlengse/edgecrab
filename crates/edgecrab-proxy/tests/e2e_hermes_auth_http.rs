//! E2E: Mode A via Hermes-format auth.json (`adapter: hermes_auth`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use edgecrab_core::{ForwardAdapterKind, ForwardUpstreamConfig, ProxyConfig};
use edgecrab_proxy::backend::forwarder::build_forwarder_client;
use edgecrab_proxy::resolve::build_forward_adapters;
use edgecrab_proxy::server::{ProxyState, build_router};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
struct UpstreamCapture {
    auth: Arc<Mutex<Option<String>>>,
}

async fn upstream_chat(
    State(cap): State<UpstreamCapture>,
    headers: HeaderMap,
    _body: Bytes,
) -> impl IntoResponse {
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    *cap.auth.lock().expect("lock") = auth;
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "from-upstream"}}]
        })),
    )
}

#[tokio::test]
async fn e2e_hermes_auth_file_adapter_forwards_agent_key() {
    edgecrab_proxy::enable_e2e_direct_http();
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

    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    std::fs::write(
        &auth_path,
        serde_json::json!({
            "version": 2,
            "providers": {
                "nous": {
                    "agent_key": "hermes-inference-jwt",
                    "base_url": upstream_base
                }
            }
        })
        .to_string(),
    )
    .expect("write auth");

    let mut cfg = ProxyConfig::default();
    cfg.model_aliases
        .insert("nous-chat".into(), "forward:nous".into());
    cfg.forward_upstreams.insert(
        "nous".into(),
        ForwardUpstreamConfig {
            base_url: upstream_base.clone(),
            adapter: ForwardAdapterKind::HermesAuth,
            auth_provider: Some("nous".into()),
            auth_path: Some(auth_path),
            bearer: None,
            bearer_env: None,
            auth_hint: None,
        },
    );
    let forward_upstreams = cfg.forward_upstreams.clone();
    let state = ProxyState {
        token: "proxy-client-token".into(),
        config: cfg,
        default_model_spec: None,
        forward_adapters: build_forward_adapters(&forward_upstreams),
        forward_client: Arc::new(build_forwarder_client().expect("client")),
        forward_only: None,
    };
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let proxy_base = format!("http://{}", listener.local_addr().expect("addr"));
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("proxy");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "nous-chat",
        "messages": [{"role": "user", "content": "hi"}]
    });
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer proxy-client-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);

    let upstream_auth = capture.auth.lock().expect("lock").clone().expect("auth");
    assert_eq!(upstream_auth, "Bearer hermes-inference-jwt");
}
