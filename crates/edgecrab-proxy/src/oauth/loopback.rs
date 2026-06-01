//! Ephemeral 127.0.0.1 HTTP listener for OAuth redirect callbacks.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::error::ProxyError;

pub const LOOPBACK_HOST: &str = "127.0.0.1";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "error_description")]
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OAuthCallback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Clone)]
struct CallbackState {
    result: Arc<Mutex<OAuthCallback>>,
    expected_path: String,
}

async fn handle_oauth_callback(
    State(state): State<CallbackState>,
    Query(query): Query<OAuthCallbackQuery>,
    method: Method,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> Response {
    if uri.path() != state.expected_path {
        return (StatusCode::NOT_FOUND, "Not found.").into_response();
    }

    if method == Method::OPTIONS {
        let mut builder = Response::builder().status(StatusCode::NO_CONTENT);
        if let Some(origin) = headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .filter(|o| matches!(*o, "https://accounts.x.ai" | "https://auth.x.ai"))
        {
            builder = builder
                .header("Access-Control-Allow-Origin", origin)
                .header("Access-Control-Allow-Methods", "GET, OPTIONS")
                .header("Access-Control-Allow-Headers", "Content-Type")
                .header("Access-Control-Allow-Private-Network", "true")
                .header("Vary", "Origin");
        }
        return match builder.body(axum::body::Body::empty()) {
            Ok(resp) => resp.into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    let incoming = OAuthCallback {
        code: query.code.clone(),
        state: query.state.clone(),
        error: query.error.clone(),
        error_description: query.error_description.clone(),
    };

    if incoming.code.is_none() && incoming.error.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Html(
                "<html><body><h1>Authorization not received.</h1>\
                 <p>No authorization code in this callback URL.</p></body></html>",
            ),
        )
            .into_response();
    }

    {
        let mut guard = state.result.lock().await;
        if guard.code.is_none() && guard.error.is_none() {
            *guard = incoming;
        }
    }

    if query.error.is_some() {
        (
            StatusCode::OK,
            Html("<html><body><h1>Authorization failed.</h1>You can close this tab.</body></html>"),
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            Html("<html><body><h1>Authorization received.</h1>You can close this tab.</body></html>"),
        )
            .into_response()
    }
}

pub struct LoopbackServer {
    pub redirect_uri: String,
    result: Arc<Mutex<OAuthCallback>>,
    cancel: CancellationToken,
    join: JoinHandle<()>,
}

impl LoopbackServer {
    pub async fn start(preferred_port: u16, expected_path: &str) -> Result<Self, ProxyError> {
        let path = if expected_path.is_empty() {
            "/callback".to_string()
        } else if expected_path.starts_with('/') {
            expected_path.to_string()
        } else {
            format!("/{expected_path}")
        };

        let result = Arc::new(Mutex::new(OAuthCallback::default()));
        let state = CallbackState {
            result: result.clone(),
            expected_path: path.clone(),
        };

        let mut last_err = None;
        let mut listener = None;
        for port in [preferred_port, 0] {
            let addr: SocketAddr = format!("{LOOPBACK_HOST}:{port}")
                .parse()
                .map_err(|e| ProxyError::UpstreamAuth(format!("loopback addr: {e}")))?;
            match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => {
                    listener = Some(l);
                    break;
                }
                Err(e) => last_err = Some(e),
            }
        }
        let listener = listener.ok_or_else(|| {
            ProxyError::UpstreamAuth(format!(
                "could not bind loopback OAuth server: {last_err:?}"
            ))
        })?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| ProxyError::UpstreamAuth(format!("loopback local_addr: {e}")))?;
        let redirect_uri = format!("http://{LOOPBACK_HOST}:{}{}", local_addr.port(), path);

        let app = Router::new()
            .route(&path, any(handle_oauth_callback))
            .with_state(state);

        let cancel = CancellationToken::new();
        let cancel_serve = cancel.clone();
        let join = tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    cancel_serve.cancelled().await;
                })
                .await;
        });

        Ok(Self {
            redirect_uri,
            result,
            cancel,
            join,
        })
    }

    pub async fn wait_for_callback(
        &self,
        timeout: Duration,
    ) -> Result<OAuthCallback, ProxyError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            {
                let guard = self.result.lock().await;
                if guard.code.is_some() || guard.error.is_some() {
                    return Ok(guard.clone());
                }
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(ProxyError::UpstreamAuth(
            "timed out waiting for OAuth callback on loopback".into(),
        ))
    }

    pub async fn shutdown(self) {
        self.cancel.cancel();
        let _ = self.join.await;
    }
}

pub fn validate_loopback_redirect_uri(redirect_uri: &str) -> Result<(), ProxyError> {
    let parsed = url::Url::parse(redirect_uri)
        .map_err(|e| ProxyError::UpstreamAuth(format!("invalid redirect_uri: {e}")))?;
    if parsed.scheme() != "http" {
        return Err(ProxyError::UpstreamAuth(
            "xAI OAuth redirect_uri must use http://127.0.0.1".into(),
        ));
    }
    if parsed.host_str() != Some(LOOPBACK_HOST) {
        return Err(ProxyError::UpstreamAuth(
            "xAI OAuth redirect_uri must point to 127.0.0.1".into(),
        ));
    }
    if parsed.port().is_none() {
        return Err(ProxyError::UpstreamAuth(
            "xAI OAuth redirect_uri must include an explicit port".into(),
        ));
    }
    Ok(())
}
