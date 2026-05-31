//! E2E: Nous Portal quarantines auth on terminal refresh failure (Hermes parity).

use edgecrab_proxy::{
    enable_e2e_direct_http, resolve_nous_credentials_async, state_requires_relogin,
    DEFAULT_NOUS_INFERENCE,
};

#[tokio::test]
async fn e2e_nous_portal_quarantines_on_invalid_grant() {
    enable_e2e_direct_http();
    let dir = tempfile::tempdir().expect("tempdir");
    let auth_path = dir.path().join("auth.json");
    let portal_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let portal_addr = portal_listener.local_addr().expect("addr");
    let portal_base = format!("http://{portal_addr}");
    let portal_app = axum::Router::new().route(
        "/api/oauth/token",
        axum::routing::post(|| async {
            (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_grant",
                    "error_description": "refresh token revoked"
                })),
            )
        }),
    );
    tokio::spawn(async move {
        axum::serve(portal_listener, portal_app)
            .await
            .expect("portal");
    });
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    std::fs::write(
        &auth_path,
        serde_json::json!({
            "providers": {
                "nous": {
                    "refresh_token": "rt-dead",
                    "portal_base_url": portal_base
                }
            },
            "credential_pool": {
                "nous": [{"refresh_token": "rt-pool-dead"}]
            }
        })
        .to_string(),
    )
    .expect("write auth");

    let err = resolve_nous_credentials_async(
        &auth_path,
        "nous",
        DEFAULT_NOUS_INFERENCE,
        false,
    )
    .await
    .expect_err("refresh should fail");

    assert!(
        err.to_string().contains("invalid_grant") || err.to_string().contains("re-authenticate"),
        "unexpected error: {err}"
    );

    let persisted: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&auth_path).expect("read")).expect("json");
    let state = &persisted["providers"]["nous"];
    assert!(state_requires_relogin(state));
    assert!(state.get("refresh_token").is_none());
    assert!(persisted["credential_pool"].get("nous").is_none());
}
