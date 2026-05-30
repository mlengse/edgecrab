//! DDGS error mapping — scrape failures must not trigger chain API rate-limit cooldown.

use super::settings::DdgsEngine;
use crate::tools::web::search::error::SearchError;

/// Map HTTP status from a metasearch engine (never expose raw URLs to users).
pub fn map_engine_http_status(
    backend: &str,
    engine: DdgsEngine,
    code: u16,
) -> SearchError {
    let label = engine.label();
    match code {
        429 => SearchError::server(
            backend,
            503,
            format!("{label} rate-limited this request — try another engine or set DDGS_PROXY."),
        ),
        403 | 418 => SearchError::server(
            backend,
            503,
            format!("{label} blocked this request (bot challenge or forbidden)."),
        ),
        202 | 301 | 400 => SearchError::server(
            backend,
            503,
            format!("{label} temporarily unavailable (HTTP {code})."),
        ),
        408 => SearchError::timeout(backend, format!("{label} request timed out.")),
        500..=599 => SearchError::server(backend, code, format!("{label} server error (HTTP {code}).")),
        400..=499 => SearchError::bad_request(backend, code, format!("{label} rejected request (HTTP {code}).")),
        _ => SearchError::hard(backend, format!("{label} unexpected HTTP {code}.")),
    }
}

/// Network/transport failure — safe user-facing text (no URLs).
pub fn map_transport_error(backend: &str, engine: DdgsEngine, err: &str) -> SearchError {
    let lower = err.to_ascii_lowercase();
    if lower.contains("time") {
        SearchError::timeout(backend, format!("{} request timed out.", engine.label()))
    } else {
        SearchError::network(
            backend,
            format!("{} network error.", engine.label()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::web::search::error::SearchErrorKind;

    #[test]
    fn scrape_403_is_server_not_chain_rate_limit() {
        let err = map_engine_http_status("ddgs", DdgsEngine::Html, 403);
        assert!(matches!(err.kind, SearchErrorKind::Server(503)));
        assert!(!err.message.contains("duckduckgo.com"));
    }

    #[test]
    fn http_202_is_not_mapped_at_transport_when_body_parsed() {
        // Transport accepts 202 and lets detect/parse decide; 301 still maps here.
        let err = map_engine_http_status("ddgs", DdgsEngine::Lite, 301);
        assert!(matches!(err.kind, SearchErrorKind::Server(503)));
    }

    #[test]
    fn api_429_from_engine_is_server_not_rate_limit_kind() {
        let err = map_engine_http_status("ddgs", DdgsEngine::Bing, 429);
        assert!(matches!(err.kind, SearchErrorKind::Server(503)));
    }
}
