//! Tool response shaping — native EdgeCrab + Hermes-compatible envelope.

use serde_json::{Value, json};

use crate::config_ref::WebSearchConfigRef;

use super::backend::SearchResult;
use super::backend_settings::{backend_is_configured, lookup_backend_config};

const PAID_SEARCH_BACKENDS: &[&str] = &[
    "tavily",
    "brave",
    "firecrawl",
    "searxng",
    "exa",
    "parallel",
    "xai",
];

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
    skipped_tool_override: Option<&str>,
    note: Option<&str>,
    results: &[SearchResult],
) -> Value {
    json!({
        "success": true,
        "query": query,
        "backend": backend,
        "fallback_from": fallback_from,
        "skipped_tool_override": skipped_tool_override,
        "note": note,
        "results": results,
        "data": {
            "web": hermes_web_rows(results),
        }
    })
}

/// Note when the agent requested an unconfigured backend and the config chain was used.
pub fn web_search_skipped_override_note(
    skipped: Option<&str>,
    chain_summary: &str,
) -> Option<String> {
    let name = skipped?.trim();
    if name.is_empty() {
        return None;
    }
    Some(format!(
        "Ignored unconfigured backend '{name}' — used configured chain ({chain_summary})."
    ))
}

/// Join optional note lines for the agent-facing `note` field.
pub fn merge_web_search_notes(parts: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    let merged: Vec<String> = parts.into_iter().flatten().collect();
    if merged.is_empty() {
        None
    } else {
        Some(merged.join(" "))
    }
}

/// Agent-facing notes for a successful search (skipped override + fallback hints).
pub fn build_web_search_agent_notes(
    used_backend: &str,
    fallback_from: Option<&str>,
    skipped_tool_override: Option<&str>,
    chain_summary: &str,
    cfg: &WebSearchConfigRef,
) -> Option<String> {
    merge_web_search_notes([
        web_search_skipped_override_note(skipped_tool_override, chain_summary),
        web_search_result_note(used_backend, fallback_from, cfg),
    ])
}

/// TUI one-liner fragment: `via tavily` or `via ddgs (fallback from tavily)`.
pub fn summarize_web_search_backend(used_backend: &str, fallback_from: Option<&str>) -> String {
    match fallback_from.filter(|name| !name.is_empty()) {
        Some(primary) => format!("via {used_backend} (fallback from {primary})"),
        None => format!("via {used_backend}"),
    }
}

/// TUI/status line: `N result(s) via backend …`, optionally prefixed when a tool override was skipped.
pub fn format_web_search_status_line(
    count: usize,
    used_backend: &str,
    fallback_from: Option<&str>,
    skipped_tool_override: Option<&str>,
) -> String {
    let core = format_web_search_result_count(count, used_backend, fallback_from);
    match skipped_tool_override.filter(|s| !s.is_empty()) {
        Some(skipped) => format!("(ignored {skipped}) {core}"),
        None => core,
    }
}

/// TUI/status line: `N result(s) via backend …`.
pub fn format_web_search_result_count(
    count: usize,
    used_backend: &str,
    fallback_from: Option<&str>,
) -> String {
    let count_part = if count == 1 {
        "1 result".to_string()
    } else {
        format!("{count} results")
    };
    format!(
        "{count_part} {}",
        summarize_web_search_backend(used_backend, fallback_from)
    )
}

/// Optional agent-facing note on ddgs usage — suppresses misleading hints when keys exist.
pub fn web_search_result_note(
    used_backend: &str,
    fallback_from: Option<&str>,
    cfg: &WebSearchConfigRef,
) -> Option<String> {
    if used_backend != "ddgs" {
        return None;
    }
    if let Some(primary) = fallback_from.filter(|name| !name.is_empty()) {
        return Some(format!(
            "Primary backend '{primary}' failed; fell back to DuckDuckGo (ddgs)."
        ));
    }
    let paid_configured = PAID_SEARCH_BACKENDS
        .iter()
        .any(|name| backend_is_configured(name, &lookup_backend_config(&cfg.backends, name)));
    if paid_configured {
        return None;
    }
    Some(
        "DuckDuckGo (ddgs) is the no-key fallback. \
         For reliable broad search set SEARXNG_URL, BRAVE_API_KEY, or TAVILY_API_KEY."
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_isolation::web_config_test_lock;
    use super::*;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        web_config_test_lock()
    }

    #[test]
    fn summarize_includes_fallback_from() {
        assert_eq!(
            summarize_web_search_backend("ddgs", Some("tavily")),
            "via ddgs (fallback from tavily)"
        );
        assert_eq!(summarize_web_search_backend("tavily", None), "via tavily");
    }

    #[test]
    fn ddgs_note_on_fallback_mentions_primary() {
        let _lock = env_lock();
        let cfg = WebSearchConfigRef::default();
        let note = web_search_result_note("ddgs", Some("tavily"), &cfg).expect("note");
        assert!(note.contains("tavily"));
        assert!(note.contains("fell back"));
    }

    #[test]
    fn ddgs_note_suppressed_when_paid_backend_configured() {
        let _lock = env_lock();
        let prev = std::env::var("TAVILY_API_KEY").ok();
        unsafe { std::env::set_var("TAVILY_API_KEY", "test-key") };
        let cfg = WebSearchConfigRef::default();
        assert!(web_search_result_note("ddgs", None, &cfg).is_none());
        unsafe { std::env::remove_var("TAVILY_API_KEY") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("TAVILY_API_KEY", v) };
        }
    }

    #[test]
    fn ddgs_note_shown_when_no_paid_backends() {
        let _lock = env_lock();
        let keys = [
            "TAVILY_API_KEY",
            "BRAVE_API_KEY",
            "BRAVE_SEARCH_API_KEY",
            "FIRECRAWL_API_KEY",
            "SEARXNG_URL",
            "EXA_API_KEY",
            "PARALLEL_API_KEY",
            "XAI_API_KEY",
        ];
        let saved: Vec<_> = keys
            .iter()
            .filter_map(|key| std::env::var(key).ok().map(|v| (*key, v)))
            .collect();
        for key in keys {
            unsafe { std::env::remove_var(key) };
        }
        let cfg = WebSearchConfigRef::default();
        let note = web_search_result_note("ddgs", None, &cfg).expect("note");
        assert!(note.contains("no-key fallback"));
        for key in keys {
            unsafe { std::env::remove_var(key) };
        }
        for (key, value) in saved {
            unsafe { std::env::set_var(key, value) };
        }
    }

    #[test]
    fn skipped_override_note_mentions_backend() {
        let note =
            web_search_skipped_override_note(Some("parallel"), "tavily → ddgs").expect("note");
        assert!(note.contains("parallel"));
        assert!(note.contains("tavily → ddgs"));
    }

    #[test]
    fn merge_notes_joins_non_empty() {
        let merged = merge_web_search_notes([Some("first.".into()), None, Some("second.".into())]);
        assert_eq!(merged.as_deref(), Some("first. second."));
    }

    #[test]
    fn status_line_prefixes_skipped_override() {
        assert_eq!(
            format_web_search_status_line(3, "ddgs", None, Some("parallel")),
            "(ignored parallel) 3 results via ddgs"
        );
    }

    #[test]
    fn format_count_with_fallback() {
        assert_eq!(
            format_web_search_result_count(5, "ddgs", Some("tavily")),
            "5 results via ddgs (fallback from tavily)"
        );
    }
}
