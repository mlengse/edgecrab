#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: Hermes-style setup schema rows for all registered web providers.

mod common;

use common::registry_guard;
use edgecrab_tools::tools::web::search::{
    get_web_search_backend, list_web_provider_setup_schemas,
    provider_capabilities::{supports_crawl, supports_extract, supports_search},
};

#[test]
fn e2e_each_registered_backend_has_setup_schema() {
    let _lock = registry_guard();
    let schemas = list_web_provider_setup_schemas();
    assert!(
        schemas.len() >= 8,
        "expected at least 8 builtin backends, got {}",
        schemas.len()
    );

    for (name, schema) in &schemas {
        assert!(!schema.name.is_empty(), "{name}: schema.name");
        let backend = get_web_search_backend(name).expect("{name} in registry");
        assert_eq!(backend.name(), name.as_str());
        if supports_search(name) && supports_extract(name) {
            assert!(
                !schema.env_vars.is_empty() || name == "ddgs",
                "{name}: paid extract/search providers should document env vars"
            );
        }
    }
}

#[test]
fn e2e_crawl_capability_matches_hermes() {
    let _lock = registry_guard();
    // Hermes: only Firecrawl + Tavily expose crawl APIs on providers.
    assert!(supports_crawl("firecrawl"));
    assert!(supports_crawl("tavily"));
    assert!(!supports_crawl("exa"));
    assert!(!supports_crawl("parallel"));
    assert!(!supports_crawl("searxng"));
}
