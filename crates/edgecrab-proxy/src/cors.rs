//! Optional CORS for browser-based clients (opt-in via `proxy.cors_allow_origins`).

use axum::extract::Request;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use edgecrab_core::ProxyConfig;

#[derive(Clone)]
pub struct CorsState {
    pub allowed_origins: Vec<String>,
}

impl From<&ProxyConfig> for CorsState {
    fn from(cfg: &ProxyConfig) -> Self {
        Self {
            allowed_origins: cfg.cors_allow_origins.clone(),
        }
    }
}

fn origin_allowed(state: &CorsState, origin: &str) -> bool {
    state
        .allowed_origins
        .iter()
        .any(|o| o == "*" || o == origin)
}

fn cors_header_value(state: &CorsState, origin: &str) -> HeaderValue {
    let value = if state.allowed_origins.iter().any(|o| o == "*") {
        "*".to_string()
    } else {
        origin.to_string()
    };
    HeaderValue::from_str(&value).unwrap_or_else(|_| HeaderValue::from_static("*"))
}

/// No-op when `cors_allow_origins` is empty.
pub async fn cors_middleware(
    axum::extract::State(state): axum::extract::State<CorsState>,
    request: Request,
    next: Next,
) -> Response {
    if state.allowed_origins.is_empty() {
        return next.run(request).await;
    }

    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if request.method() == Method::OPTIONS {
        if origin.is_empty() || !origin_allowed(&state, &origin) {
            return StatusCode::FORBIDDEN.into_response();
        }
        return (
            StatusCode::NO_CONTENT,
            [
                (
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    cors_header_value(&state, &origin),
                ),
                (
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    HeaderValue::from_static("GET, POST, OPTIONS"),
                ),
                (
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    HeaderValue::from_static("authorization, content-type"),
                ),
            ],
        )
            .into_response();
    }

    let mut response = next.run(request).await;
    if !origin.is_empty() && origin_allowed(&state, &origin) {
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            cors_header_value(&state, &origin),
        );
    }
    response
}
