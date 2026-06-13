//! HTTP-level e2e tests for the OpenAI-compatible proxy.

use std::sync::Arc;
use std::time::Duration;

use edgecrab_core::ProxyConfig;
use edgecrab_proxy::backend::forwarder::build_forwarder_client;
use edgecrab_proxy::resolve::build_forward_adapters;
use edgecrab_proxy::server::{ProxyState, build_router};
use serde_json::Value;
use tokio::net::TcpListener;

async fn spawn_test_proxy(token: &str) -> (String, tokio::task::JoinHandle<()>) {
    edgecrab_proxy::enable_e2e_direct_http();
    let mut cfg = ProxyConfig::default();
    cfg.model_aliases
        .insert("mock-model".into(), "mock/test".into());
    let forward_adapters = build_forward_adapters(&cfg.forward_upstreams);
    let state = ProxyState {
        token: token.into(),
        config: cfg,
        default_model_spec: None,
        forward_adapters,
        forward_client: Arc::new(build_forwarder_client().expect("forward client")),
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

#[tokio::test]
async fn e2e_openai_shape_non_streaming() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "mock-model",
        "messages": [{"role": "user", "content": "ping"}],
        "stream": false
    });
    let resp = client
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", "Bearer e2e-secret-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let json: Value = resp.json().await.expect("json");
    assert_eq!(json["object"], "chat.completion");
    assert!(json["choices"][0]["message"]["content"].is_string());
}

#[tokio::test]
async fn e2e_streaming_sse_contains_done() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "mock-model",
        "messages": [{"role": "user", "content": "stream"}],
        "stream": true
    });
    let resp = client
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", "Bearer e2e-secret-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    let status = resp.status();
    let text = resp.text().await.expect("body");
    assert_eq!(status, 200, "body: {text}");
    assert!(text.contains("chat.completion.chunk") || text.contains("[DONE]"));
    assert!(text.contains("[DONE]"));
}

#[tokio::test]
async fn e2e_models_list_requires_auth() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let denied = client
        .get(format!("{base}/v1/models"))
        .send()
        .await
        .expect("request");
    assert_eq!(denied.status(), 401);

    let ok = client
        .get(format!("{base}/v1/models"))
        .header("Authorization", "Bearer e2e-secret-token")
        .send()
        .await
        .expect("request");
    assert_eq!(ok.status(), 200);
}

#[tokio::test]
async fn e2e_malformed_bearer_returns_401_not_500() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "mock-model",
        "messages": [{"role": "user", "content": "hi"}]
    });
    let resp = client
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", "NotBearer garbage")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 401);
    let json: Value = resp.json().await.expect("json");
    assert_eq!(json["error"]["code"], "invalid_api_key");
}

#[tokio::test]
async fn e2e_unknown_model_returns_404_openai_shape() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "definitely-not-configured",
        "messages": [{"role": "user", "content": "hi"}]
    });
    let resp = client
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", "Bearer e2e-secret-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 404);
    let json: Value = resp.json().await.expect("json");
    assert_eq!(json["error"]["code"], "model_not_found");
}

#[tokio::test]
async fn e2e_chat_with_tools_accepts_openai_tools_schema() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "mock-model",
        "messages": [{"role": "user", "content": "weather?"}],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {"type": "object", "properties": {}}
            }
        }],
        "tool_choice": "auto",
        "stream": false
    });
    let resp = client
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", "Bearer e2e-secret-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn e2e_models_list_returns_configured_aliases() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let resp = client
        .get(format!("{base}/v1/models"))
        .header("Authorization", "Bearer e2e-secret-token")
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let json: Value = resp.json().await.expect("json");
    assert_eq!(json["object"], "list");
    let ids: Vec<_> = json["data"]
        .as_array()
        .expect("data")
        .iter()
        .filter_map(|m| m["id"].as_str())
        .collect();
    assert!(ids.contains(&"mock-model"));
}

#[tokio::test]
async fn e2e_provider_embeddings_returns_501() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "mock-model",
        "input": "hello"
    });
    let resp = client
        .post(format!("{base}/v1/embeddings"))
        .header("Authorization", "Bearer e2e-secret-token")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 501);
}

#[tokio::test]
async fn e2e_cors_preflight_when_origins_configured() {
    let mut cfg = ProxyConfig::default();
    cfg.model_aliases
        .insert("mock-model".into(), "mock/test".into());
    cfg.cors_allow_origins = vec!["http://localhost:3000".into()];
    let forward_adapters = build_forward_adapters(&cfg.forward_upstreams);
    let state = ProxyState {
        token: "e2e-secret-token".into(),
        config: cfg,
        default_model_spec: None,
        forward_adapters,
        forward_client: Arc::new(build_forwarder_client().expect("forward client")),
        forward_only: None,
    };
    let app = build_router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let base = format!("http://{addr}");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = edgecrab_proxy::e2e_http_client();
    let resp = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{base}/v1/chat/completions"),
        )
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .expect("preflight");
    assert_eq!(resp.status(), 204);
    assert!(resp.headers().contains_key("access-control-allow-origin"));
}

/// OpenAI Python SDK / LiteLLM-style request envelope (wire acceptance without live keys).
#[tokio::test]
async fn e2e_openai_sdk_shaped_request_accepted() {
    let (base, _handle) = spawn_test_proxy("e2e-secret-token").await;
    let client = edgecrab_proxy::e2e_http_client();
    let body = serde_json::json!({
        "model": "mock-model",
        "messages": [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Hello"}
        ],
        "temperature": 0.7,
        "max_tokens": 128,
        "stream": false,
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"]
                }
            }
        }],
        "tool_choice": "auto"
    });
    let resp = client
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", "Bearer e2e-secret-token")
        .header("OpenAI-Beta", "assistants=v2")
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let json: Value = resp.json().await.expect("json");
    assert_eq!(json["object"], "chat.completion");
}
