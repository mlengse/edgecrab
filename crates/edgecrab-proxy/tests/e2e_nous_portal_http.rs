//! E2E: Nous Portal adapter refreshes OAuth and forwards invoke JWT.

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
use edgecrab_proxy::backend::nous::{INFERENCE_INVOKE_SCOPE, make_jwt};
use edgecrab_proxy::resolve::build_forward_adapters;
use edgecrab_proxy::server::{ProxyState, build_router};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
struct UpstreamCapture {
    auth: Arc<Mutex<Option<String>>>,
    refresh_hits: Arc<Mutex<u32>>,
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
            "choices": [{"message": {"role": "assistant", "content": "ok"}}]
        })),
    )
}

async fn portal_refresh(
    State(cap): State<UpstreamCapture>,
    headers: HeaderMap,
) -> impl IntoResponse {
    *cap.refresh_hits.lock().expect("lock") += 1;
    let refresh = headers
        .get("x-nous-refresh-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if refresh != "rt-test" {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error": "bad_rt"})),
        );
    }
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + 7200;
    let access = make_jwt(exp, INFERENCE_INVOKE_SCOPE);
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "access_token": access,
            "refresh_token": "rt-test-rotated",
            "expires_in": 7200
        })),
    )
}

#[tokio::test]
async fn e2e_nous_portal_refreshes_and_forwards_invoke_jwt() {
    edgecrab_proxy::enable_e2e_direct_http();
    let capture = UpstreamCapture::default();

    let portal_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let portal_addr = portal_listener.local_addr().expect("addr");
    let portal_base = format!("http://{portal_addr}");
    let portal_app = Router::new()
        .route("/api/oauth/token", post(portal_refresh))
        .with_state(capture.clone());
    tokio::spawn(async move {
        axum::serve(portal_listener, portal_app)
            .await
            .expect("portal");
    });

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
    tokio::time::sleep(Duration::from_millis(80)).await;

    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    // Expired invoke JWT forces refresh against mock portal.
    let stale = make_jwt(1, INFERENCE_INVOKE_SCOPE);
    std::fs::write(
        &auth_path,
        serde_json::json!({
            "version": 2,
            "providers": {
                "nous": {
                    "portal_base_url": portal_base,
                    "inference_base_url": upstream_base,
                    "refresh_token": "rt-test",
                    "access_token": stale,
                    "client_id": "hermes-cli",
                    "scope": INFERENCE_INVOKE_SCOPE
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
            adapter: ForwardAdapterKind::NousPortal,
            auth_provider: Some("nous".into()),
            auth_path: Some(auth_path.clone()),
            ..Default::default()
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
    tokio::time::sleep(Duration::from_millis(80)).await;

    let client = edgecrab_proxy::e2e_http_client();
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer proxy-client-token")
        .json(&serde_json::json!({
            "model": "nous-chat",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);

    assert_eq!(*capture.refresh_hits.lock().expect("lock"), 1);
    let upstream_auth = capture.auth.lock().expect("lock").clone().expect("auth");
    assert!(upstream_auth.starts_with("Bearer "));
    let token = upstream_auth.strip_prefix("Bearer ").expect("bearer");
    assert!(token.contains('.'));
    assert_ne!(token, stale);

    let persisted: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&auth_path).expect("read")).expect("json");
    let agent_key = persisted["providers"]["nous"]["agent_key"]
        .as_str()
        .expect("agent_key");
    assert_eq!(agent_key, token);
}
