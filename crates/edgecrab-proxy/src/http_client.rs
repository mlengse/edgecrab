//! Shared reqwest client builders (DRY for forwarder + OAuth refresh).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use edgecrab_security::proxy::{apply_proxy_to_builder, resolve_proxy_url};
use reqwest::Client;

use crate::error::ProxyError;

static E2E_DIRECT_HTTP: AtomicBool = AtomicBool::new(false);

pub fn e2e_direct_http_enabled() -> bool {
    E2E_DIRECT_HTTP.load(Ordering::SeqCst)
}

/// Enable direct HTTP for mock stacks (integration tests on `127.0.0.1`).
pub fn enable_e2e_direct_http() {
    E2E_DIRECT_HTTP.store(true, Ordering::SeqCst);
}

fn finish_builder(builder: reqwest::ClientBuilder) -> Result<Client, ProxyError> {
    builder
        .build()
        .map_err(|e| ProxyError::Upstream(format!("HTTP client: {e}")))
}

/// OAuth / refresh clients (short timeout, limited redirects).
pub fn build_oauth_http_client(timeout: Duration) -> Result<Client, ProxyError> {
    let timeout = if e2e_direct_http_enabled() {
        timeout.min(Duration::from_secs(5))
    } else {
        timeout
    };
    let builder = Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(5));
    if e2e_direct_http_enabled() {
        finish_builder(builder.no_proxy())
    } else {
        finish_builder(apply_proxy_to_builder(
            builder,
            resolve_proxy_url(None).as_deref(),
        ))
    }
}

/// Forwarder client (long timeout, streams upstream bodies).
pub fn build_forwarder_http_client() -> Result<Client, ProxyError> {
    let builder = Client::builder().timeout(Duration::from_secs(300));
    if e2e_direct_http_enabled() {
        finish_builder(builder.no_proxy())
    } else {
        finish_builder(apply_proxy_to_builder(
            builder,
            resolve_proxy_url(None).as_deref(),
        ))
    }
}
