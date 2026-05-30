//! Hermes-aligned capability flags per backend name.
//!
//! Search-only providers (`brave`, `ddgs`, `searxng`, `xai`) must not be
//! selected for `web_extract` / `web_crawl` — config falls through to auto;
//! explicit tool/env overrides surface a typed "search-only" error.

use super::backend_settings::normalize_backend_name;
use super::registry::get_web_search_backend;

/// Backends that implement `web_search`.
pub fn supports_search(name: &str) -> bool {
    let name = normalize_backend_name(name);
    if let Some(backend) = get_web_search_backend(&name) {
        return backend.supports_search();
    }
    !matches!(name.as_str(), "native" | "browser")
}

/// Backends that implement paid/local `web_extract` (excluding auto-chain `native`).
pub fn supports_extract(name: &str) -> bool {
    let name = normalize_backend_name(name);
    if let Some(backend) = get_web_search_backend(&name) {
        return backend.supports_extract();
    }
    matches!(name.as_str(), "native" | "browser")
}

pub fn supports_crawl(name: &str) -> bool {
    let name = normalize_backend_name(name);
    get_web_search_backend(&name)
        .map(|b| b.supports_crawl())
        .unwrap_or(false)
}

pub fn is_search_only(name: &str) -> bool {
    supports_search(name) && !supports_extract(name)
}

pub fn search_only_error_message(name: &str, tool: &str) -> String {
    let name = normalize_backend_name(name);
    let label = match name.as_str() {
        "brave" => "Brave Search",
        "ddgs" => "DuckDuckGo",
        "searxng" => "SearXNG",
        "xai" => "xAI",
        other => other,
    };
    format!(
        "{label} is a search-only backend and cannot be used for {tool}. \
         Set web.extract_backend to firecrawl, parallel, tavily, or exa, or omit backend for auto."
    )
}

#[cfg(test)]
mod tests {
    use super::super::registry::{reset_registry_for_tests, test_registry_lock};
    use super::*;

    #[test]
    fn paid_providers_support_both_capabilities() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        for name in ["firecrawl", "parallel", "tavily", "exa"] {
            assert!(supports_search(name), "{name} search");
            assert!(supports_extract(name), "{name} extract");
        }
    }

    #[test]
    fn free_search_providers_are_search_only() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        for name in ["searxng", "brave", "ddgs", "xai"] {
            assert!(supports_search(name), "{name} search");
            assert!(!supports_extract(name), "{name} extract");
            assert!(is_search_only(name));
        }
    }

    #[test]
    fn native_and_browser_are_extract_only() {
        assert!(!supports_search("native"));
        assert!(supports_extract("native"));
        assert!(!supports_search("browser"));
        assert!(supports_extract("browser"));
    }

    #[test]
    fn crawl_capable_providers() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        for name in ["firecrawl", "tavily"] {
            assert!(supports_crawl(name), "{name} crawl");
        }
        for name in ["searxng", "brave", "ddgs", "xai", "exa", "parallel"] {
            assert!(!supports_crawl(name), "{name} crawl");
        }
    }

    #[test]
    fn registry_ddgs_display_name_is_human_readable() {
        let _lock = super::super::registry::test_registry_lock();
        super::super::registry::reset_registry_for_tests();
        let backend = super::super::registry::get_web_search_backend("ddgs").expect("ddgs");
        assert_eq!(backend.display_name(), "DuckDuckGo (ddgs)");
    }
}
