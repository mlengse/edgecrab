//! E2E: Grok / xAI OAuth flows (`edgecrab proxy setup grok` + `--provider xai`).

use edgecrab_proxy::e2e_harness::{
    e2e_http_client, grok_recipe_proxy_config, spawn_proxy, spawn_xai_mock_stack,
    upstream_stream, write_xai_auth_json, UpstreamCapture,
};
use edgecrab_proxy::guide::{apply_recipe, resolve_recipe, RECIPE_XAI};

#[tokio::test]
async fn e2e_grok_setup_recipe_refreshes_oauth_and_forwards() {
    edgecrab_proxy::enable_e2e_direct_http();
    let (token_url, upstream_base, capture) = spawn_xai_mock_stack().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    write_xai_auth_json(&auth_path, &token_url);

    let cfg = grok_recipe_proxy_config(&upstream_base, auth_path.clone());
    assert_eq!(cfg.model_aliases.get("grok"), Some(&"forward:xai".to_string()));

    let (proxy_base, _handle) = spawn_proxy(cfg, None, "proxy-client-token").await;
    let client = e2e_http_client();
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer proxy-client-token")
        .json(&serde_json::json!({
            "model": "grok",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    assert_eq!(*capture.refresh_hits.lock().expect("lock"), 1);
    assert_eq!(
        capture.auth.lock().expect("lock").as_deref(),
        Some("Bearer fresh-xai-access")
    );

    let persisted: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&auth_path).expect("read")).expect("json");
    assert_eq!(
        persisted["providers"]["xai-oauth"]["tokens"]["access_token"],
        "fresh-xai-access"
    );
}

#[tokio::test]
async fn e2e_grok_models_list_exposes_grok_alias() {
    edgecrab_proxy::enable_e2e_direct_http();
    let (token_url, upstream_base, _capture) = spawn_xai_mock_stack().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    edgecrab_proxy::e2e_harness::write_xai_auth_json_with_access(
        &auth_path,
        &token_url,
        Some("cached-xai-access"),
    );

    let mut cfg = grok_recipe_proxy_config(&upstream_base, auth_path);
    cfg.default_forward_upstream = None;
    let (proxy_base, _handle) = spawn_proxy(cfg, None, "proxy-client-token").await;

    let client = e2e_http_client();
    let resp = client
        .get(format!("{proxy_base}/v1/models"))
        .header("Authorization", "Bearer proxy-client-token")
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.expect("json");
    let ids: Vec<String> = json["data"]
        .as_array()
        .expect("data array")
        .iter()
        .filter_map(|m| m["id"].as_str().map(str::to_string))
        .collect();
    assert!(ids.contains(&"grok".to_string()), "expected grok alias in {ids:?}");
}

#[tokio::test]
async fn e2e_xai_forward_only_provider_ignores_model_and_refreshes() {
    edgecrab_proxy::enable_e2e_direct_http();
    let (token_url, upstream_base, capture) = spawn_xai_mock_stack().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    write_xai_auth_json(&auth_path, &token_url);

    let cfg = grok_recipe_proxy_config(&upstream_base, auth_path);
    let (proxy_base, _handle) = spawn_proxy(cfg, Some("xai".into()), "proxy-client-token").await;

    let client = e2e_http_client();
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer proxy-client-token")
        .json(&serde_json::json!({
            "model": "any-model-ignored-by-forward-only",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    assert_eq!(*capture.refresh_hits.lock().expect("lock"), 1);
    assert_eq!(
        capture.auth.lock().expect("lock").as_deref(),
        Some("Bearer fresh-xai-access")
    );
}

#[tokio::test]
async fn e2e_grok_streaming_sse_passes_through_with_xai_oauth() {
    edgecrab_proxy::enable_e2e_direct_http();
    let capture = UpstreamCapture::default();

    let oauth_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let oauth_addr = oauth_listener.local_addr().expect("addr");
    let token_url = format!("http://{oauth_addr}/oauth/token");
    let oauth_app = axum::Router::new()
        .route(
            "/oauth/token",
            axum::routing::post(edgecrab_proxy::e2e_harness::oauth_refresh),
        )
        .with_state(capture.clone());
    tokio::spawn(async move {
        axum::serve(oauth_listener, oauth_app)
            .await
            .expect("oauth");
    });

    let upstream_app = axum::Router::new()
        .route(
            "/v1/chat/completions",
            axum::routing::post(upstream_stream),
        )
        .with_state(capture.clone());
    let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let upstream_addr = upstream_listener.local_addr().expect("addr");
    let upstream_base = format!("http://{upstream_addr}/v1");
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_app)
            .await
            .expect("upstream");
    });
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    write_xai_auth_json(&auth_path, &token_url);

    let cfg = grok_recipe_proxy_config(&upstream_base, auth_path);
    let (proxy_base, _handle) = spawn_proxy(cfg, Some("xai".into()), "proxy-client-token").await;

    let client = e2e_http_client();
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer proxy-client-token")
        .json(&serde_json::json!({
            "model": "grok",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let text = resp.text().await.expect("body");
    assert!(text.contains("data:"));
    assert!(text.contains("[DONE]"));
    assert_eq!(
        capture.auth.lock().expect("lock").as_deref(),
        Some("Bearer fresh-xai-access")
    );
}

#[test]
fn grok_cli_recipe_resolution_matches_setup_grok() {
    assert_eq!(resolve_recipe("grok").map(|r| r.key), Some("xai"));
    assert_eq!(resolve_recipe("xai").map(|r| r.key), Some("xai"));
    let mut cfg = edgecrab_core::ProxyConfig::default();
    apply_recipe(&mut cfg, &RECIPE_XAI);
    let snippet = edgecrab_proxy::client_snippet(&cfg, Some(&RECIPE_XAI), "tok-test");
    assert_eq!(snippet.model_alias, "grok");
    assert!(snippet.forward_only_cmd.contains("--provider xai"));
}
