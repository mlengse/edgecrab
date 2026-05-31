//! E2E: xAI Grok OAuth adapter (`adapter: xai_oauth`) — delegates to shared harness.

use edgecrab_proxy::e2e_harness::{
    e2e_http_client, grok_recipe_proxy_config, spawn_proxy, spawn_xai_mock_stack,
    write_xai_auth_json,
};

#[tokio::test]
async fn e2e_xai_oauth_refreshes_and_forwards() {
    let (token_url, upstream_base, capture) = spawn_xai_mock_stack().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    write_xai_auth_json(&auth_path, &token_url);

    let mut cfg = grok_recipe_proxy_config(&upstream_base, auth_path.clone());
    cfg.model_aliases
        .insert("grok-chat".into(), "forward:xai".into());

    let (proxy_base, _handle) = spawn_proxy(cfg, None, "proxy-client-token").await;

    let client = e2e_http_client();
    let resp = client
        .post(format!("{proxy_base}/v1/chat/completions"))
        .header("Authorization", "Bearer proxy-client-token")
        .json(&serde_json::json!({
            "model": "grok-chat",
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
