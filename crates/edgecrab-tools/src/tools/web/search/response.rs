//! Tool response shaping — native EdgeCrab + Hermes-compatible envelope.

use serde_json::{Value, json};

use super::backend::SearchResult;

/// Hermes `web_search_tool` result rows: `{title, url, description, position}`.
pub fn hermes_web_rows(results: &[SearchResult]) -> Vec<Value> {
    results
        .iter()
        .map(|r| {
            json!({
                "title": r.title,
                "url": r.url,
                "description": r.snippet,
                "position": r.rank,
            })
        })
        .collect()
}

/// Build success JSON with both native and Hermes shapes.
pub fn success_payload(
    query: &str,
    backend: &str,
    fallback_from: Option<&str>,
    note: Option<&str>,
    results: &[SearchResult],
) -> Value {
    json!({
        "success": true,
        "query": query,
        "backend": backend,
        "fallback_from": fallback_from,
        "note": note,
        "results": results,
        "data": {
            "web": hermes_web_rows(results),
        }
    })
}
