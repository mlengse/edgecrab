//! Mode A — credential-attaching HTTP forwarder (Hermes `hermes proxy` style).
//!
//! Verbatim body pass-through; replaces client `Authorization` with upstream OAuth/API bearer.

use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::Response;
use axum::body::Bytes;
use futures::StreamExt;
use reqwest::Client;

use crate::http_client::build_forwarder_http_client;

use super::adapter::{UpstreamAdapter, UpstreamCredential};
use crate::error::ProxyError;

const HOP_BY_HOP: &[&str] = &[
    "host",
    "content-length",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "authorization",
];

/// Build an HTTP client for upstream forwarding with optional proxy cascade.
pub fn build_forwarder_client() -> Result<Client, ProxyError> {
    build_forwarder_http_client()
}

fn filter_request_headers(inbound: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (key, value) in inbound.iter() {
        let lower = key.as_str().to_ascii_lowercase();
        if HOP_BY_HOP.iter().any(|h| *h == lower) {
            continue;
        }
        if let (Ok(k), Ok(v)) = (
            HeaderName::try_from(key.as_str()),
            HeaderValue::try_from(value.as_bytes()),
        ) {
            out.insert(k, v);
        }
    }
    out
}

fn filter_response_headers(upstream: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (key, value) in upstream.iter() {
        let lower = key.as_str().to_ascii_lowercase();
        if HOP_BY_HOP.contains(&lower.as_str())
            || lower == "content-encoding"
            || lower == "content-length"
        {
            continue;
        }
        if let (Ok(k), Ok(v)) = (
            HeaderName::try_from(key.as_str()),
            HeaderValue::try_from(value.as_bytes()),
        ) {
            out.insert(k, v);
        }
    }
    out
}

/// Inbound forward request (keeps `forward_request` arity small for Clippy/SRP).
pub struct ForwardInbound<'a> {
    pub method: Method,
    pub rel_path: &'a str,
    pub query: Option<&'a str>,
    pub headers: &'a HeaderMap,
    pub body: Bytes,
}

/// Forward a request to the upstream, streaming the response body back unchanged.
pub async fn forward_request(
    client: &Client,
    adapter: &dyn UpstreamAdapter,
    inbound: ForwardInbound<'_>,
) -> Result<Response, ProxyError> {
    let rel_path = format!("/{}", inbound.rel_path.trim_start_matches('/'));
    let allowed_set = adapter.allowed_paths();
    if !allowed_set.contains(rel_path.as_str()) {
        let allowed: Vec<String> = allowed_set.into_iter().collect();
        return Err(ProxyError::PathNotAllowed {
            path: rel_path,
            allowed,
        });
    }

    let cred = adapter.get_credential().await?;
    forward_with_credential(client, adapter, inbound, &rel_path, cred).await
}

async fn forward_with_credential(
    client: &Client,
    adapter: &dyn UpstreamAdapter,
    inbound: ForwardInbound<'_>,
    rel_path: &str,
    cred: UpstreamCredential,
) -> Result<Response, ProxyError> {
    let (status, headers, response_body) = send_upstream(
        client,
        inbound.method.clone(),
        rel_path,
        inbound.query,
        inbound.headers,
        inbound.body.clone(),
        &cred,
    )
    .await?;

    if (status == StatusCode::UNAUTHORIZED.as_u16()
        || status == StatusCode::TOO_MANY_REQUESTS.as_u16())
        && let Some(retry) = adapter.get_retry_credential(&cred, status).await
    {
        let (status2, headers2, body2) = send_upstream(
            client,
            inbound.method,
            rel_path,
            inbound.query,
            inbound.headers,
            inbound.body,
            &retry,
        )
        .await?;
        return Ok(build_axum_response(status2, headers2, body2));
    }

    Ok(build_axum_response(status, headers, response_body))
}

enum UpstreamBody {
    Stream(reqwest::Response),
    Buffered(Bytes),
}

fn upstream_response_is_sse(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("event-stream"))
}

async fn send_upstream(
    client: &Client,
    method: Method,
    rel_path: &str,
    query: Option<&str>,
    inbound_headers: &HeaderMap,
    body: Bytes,
    cred: &UpstreamCredential,
) -> Result<(u16, HeaderMap, UpstreamBody), ProxyError> {
    let mut url = format!("{}{}", cred.base_url.trim_end_matches('/'), rel_path);
    if let Some(q) = query.filter(|s| !s.is_empty()) {
        url = format!("{url}?{q}");
    }

    let mut fwd = filter_request_headers(inbound_headers);
    fwd.insert(
        axum::http::header::AUTHORIZATION,
        HeaderValue::try_from(cred.authorization_header())
            .map_err(|e| ProxyError::Upstream(format!("invalid upstream auth header: {e}")))?,
    );

    let req = client.request(method, &url).headers(fwd);
    let req = if body.is_empty() {
        req
    } else {
        req.body(body)
    };

    let resp = req.send().await.map_err(|e| {
        if e.is_timeout() {
            ProxyError::UpstreamTimeout
        } else {
            ProxyError::UpstreamUnreachable(e.to_string())
        }
    })?;

    let status = resp.status().as_u16();
    let headers = filter_response_headers(resp.headers());
    let body = if upstream_response_is_sse(&headers) {
        UpstreamBody::Stream(resp)
    } else {
        let bytes = resp.bytes().await.map_err(|e| {
            ProxyError::Upstream(format!("read upstream body: {e}"))
        })?;
        UpstreamBody::Buffered(bytes)
    };
    Ok((status, headers, body))
}

fn build_axum_response(status: u16, headers: HeaderMap, upstream: UpstreamBody) -> Response {
    let mut builder = Response::builder().status(status);
    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }
    match upstream {
        UpstreamBody::Buffered(bytes) => builder
            .body(Body::from(bytes))
            .unwrap_or_else(|_| Response::new(Body::empty())),
        UpstreamBody::Stream(resp) => {
            let stream = resp.bytes_stream().map(|r| {
                r.map_err(|e| std::io::Error::other(e.to_string()))
            });
            builder
                .body(Body::from_stream(stream))
                .unwrap_or_else(|_| Response::new(Body::empty()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_upstream_url_with_query_string() {
        let mut url = format!(
            "{}{}",
            "https://api.example/v1".trim_end_matches('/'),
            "/chat/completions"
        );
        let q = "stream=true";
        url = format!("{url}?{q}");
        assert_eq!(
            url,
            "https://api.example/v1/chat/completions?stream=true"
        );
    }

    #[test]
    fn strips_authorization_from_forwarded_headers() {
        let mut inbound = HeaderMap::new();
        inbound.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer client-token"),
        );
        inbound.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let out = filter_request_headers(&inbound);
        assert!(!out.contains_key(axum::http::header::AUTHORIZATION));
        assert!(out.contains_key(axum::http::header::CONTENT_TYPE));
    }
}
