//! DRY credential + endpoint resolution for web search backends.
//!
//! Priority: `config.yaml` (`web_search.backends.<name>`) → env vars.

use crate::config_ref::WebSearchBackendConfigRef;

/// Hermes-aligned max result cap (`web_search_tool` uses 1–100).
pub const MAX_SEARCH_RESULTS: usize = 100;

/// Resolve API key: config first, then env fallbacks.
pub fn resolve_api_key(cfg: &WebSearchBackendConfigRef, env_keys: &[&str]) -> Option<String> {
    cfg.api_key
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            env_keys.iter().find_map(|key| {
                std::env::var(key)
                    .ok()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
            })
        })
}

/// Resolve base URL / endpoint: config `endpoint` first, then env.
pub fn resolve_endpoint(cfg: &WebSearchBackendConfigRef, env_keys: &[&str]) -> Option<String> {
    cfg.endpoint
        .as_ref()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            env_keys.iter().find_map(|key| {
                std::env::var(key)
                    .ok()
                    .map(|v| v.trim().trim_end_matches('/').to_string())
                    .filter(|v| !v.is_empty())
            })
        })
}

/// Per-backend timeout: config override → chain default.
pub fn resolve_timeout_secs(cfg: &WebSearchBackendConfigRef, chain_default: u64) -> u64 {
    cfg.timeout_secs.unwrap_or(chain_default).max(1)
}

/// Lookup backend config with Hermes alias keys (`brave-free` ↔ `brave`).
pub fn lookup_backend_config(
    backends: &std::collections::HashMap<String, WebSearchBackendConfigRef>,
    name: &str,
) -> WebSearchBackendConfigRef {
    let key = name.trim().to_ascii_lowercase();
    if let Some(cfg) = backends.get(&key) {
        return cfg.clone();
    }
    let alias = match key.as_str() {
        "brave" => "brave-free",
        "brave-free" => "brave",
        "ddg" | "duckduckgo" => "ddgs",
        _ => return WebSearchBackendConfigRef::default(),
    };
    backends.get(alias).cloned().unwrap_or_default()
}

/// Whether a backend has enough config/env to attempt a search.
pub fn backend_is_configured(name: &str, cfg: &WebSearchBackendConfigRef) -> bool {
    match normalize_backend_name(name).as_str() {
        "searxng" => resolve_endpoint(cfg, &["SEARXNG_URL"]).is_some(),
        "brave" => resolve_api_key(cfg, &["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"]).is_some(),
        "tavily" => resolve_api_key(cfg, &["TAVILY_API_KEY"]).is_some(),
        "firecrawl" => resolve_api_key(cfg, &["FIRECRAWL_API_KEY"]).is_some(),
        "exa" => resolve_api_key(cfg, &["EXA_API_KEY"]).is_some(),
        "parallel" => resolve_api_key(cfg, &["PARALLEL_API_KEY"]).is_some(),
        "xai" => resolve_api_key(cfg, &["XAI_API_KEY"]).is_some(),
        "ddgs" => true,
        // Plugin-registered backends have no credential gate.
        _ => true,
    }
}

/// Fail with [`SearchError::not_configured`] when credentials are missing.
pub fn require_backend_configured(
    backend: &str,
    cfg: &WebSearchBackendConfigRef,
) -> Result<(), crate::tools::web::search::error::SearchError> {
    if backend_is_configured(backend, cfg) {
        Ok(())
    } else {
        Err(crate::tools::web::search::error::SearchError::not_configured(backend))
    }
}

/// Resolve API key or return a structured not-configured error.
pub fn require_api_key(
    backend: &str,
    cfg: &WebSearchBackendConfigRef,
    env_keys: &[&str],
) -> Result<String, crate::tools::web::search::error::SearchError> {
    resolve_api_key(cfg, env_keys)
        .ok_or_else(|| crate::tools::web::search::error::SearchError::not_configured(backend))
}

/// Resolve endpoint URL or return a structured not-configured error.
pub fn require_endpoint(
    backend: &str,
    cfg: &WebSearchBackendConfigRef,
    env_keys: &[&str],
) -> Result<String, crate::tools::web::search::error::SearchError> {
    resolve_endpoint(cfg, env_keys)
        .ok_or_else(|| crate::tools::web::search::error::SearchError::not_configured(backend))
}

/// User-facing message when a backend is explicitly selected but not configured.
pub fn not_configured_message(name: &str) -> String {
    match normalize_backend_name(name).as_str() {
        "searxng" => {
            "SEARXNG_URL is not set. Set web_search.backends.searxng.endpoint or SEARXNG_URL."
                .into()
        }
        "brave" => {
            "BRAVE_API_KEY is not set. Set web_search.backends.brave.api_key or BRAVE_API_KEY."
                .into()
        }
        "tavily" => {
            "TAVILY_API_KEY is not set. Set web_search.backends.tavily.api_key or TAVILY_API_KEY."
                .into()
        }
        "firecrawl" => {
            "FIRECRAWL_API_KEY is not set. Set web_search.backends.firecrawl.api_key or FIRECRAWL_API_KEY."
                .into()
        }
        "exa" => {
            "EXA_API_KEY is not set. Set web_search.backends.exa.api_key or EXA_API_KEY."
                .into()
        }
        "parallel" => {
            "PARALLEL_API_KEY is not set. Set web_search.backends.parallel.api_key or PARALLEL_API_KEY."
                .into()
        }
        "xai" => {
            "XAI_API_KEY is not set. Set web_search.backends.xai.api_key or XAI_API_KEY."
                .into()
        }
        other => format!("Web search backend '{other}' is not configured."),
    }
}

/// Whether credentials were saved in EdgeCrab config or `~/.edgecrab/.env`.
///
/// Unlike [`backend_is_configured`], this ignores process environment variables
/// that were not placed in EdgeCrab's home (IDE shells often export API keys globally).
pub fn backend_is_enabled_in_edgecrab_home(name: &str, cfg: &WebSearchBackendConfigRef) -> bool {
    let name = normalize_backend_name(name);
    if name == "ddgs" {
        return true;
    }
    if cfg
        .api_key
        .as_ref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    if name == "searxng"
        && cfg
            .endpoint
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    {
        return true;
    }
    let dotenv = edgecrab_dotenv_var_names();
    env_keys_for_backend(&name)
        .iter()
        .any(|key| dotenv.contains(*key))
}

fn env_keys_for_backend(name: &str) -> &'static [&'static str] {
    match name {
        "searxng" => &["SEARXNG_URL"],
        "brave" => &["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"],
        "tavily" => &["TAVILY_API_KEY"],
        "firecrawl" => &["FIRECRAWL_API_KEY"],
        "exa" => &["EXA_API_KEY"],
        "parallel" => &["PARALLEL_API_KEY"],
        "xai" => &["XAI_API_KEY"],
        _ => &[],
    }
}

/// Variable names set in `~/.edgecrab/.env` (not the full process environment).
pub fn edgecrab_dotenv_var_names() -> std::collections::HashSet<String> {
    use std::collections::HashSet;
    let path = crate::config_ref::resolve_edgecrab_home().join(".env");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return HashSet::new();
    };
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('=').map(|(k, _)| k.trim().to_string()))
        .filter(|k| !k.is_empty())
        .collect()
}

/// Canonical backend name for credential lookup.
pub fn normalize_backend_name(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "brave-free" => "brave".into(),
        "duckduckgo" | "ddg" => "ddgs".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_api_key_overrides_env() {
        let _lock = crate::tools::web::search::registry::test_registry_lock();
        let prev = std::env::var("TAVILY_API_KEY").ok();
        unsafe { std::env::set_var("TAVILY_API_KEY", "env-key") };
        let cfg = WebSearchBackendConfigRef {
            api_key: Some("cfg-key".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_api_key(&cfg, &["TAVILY_API_KEY"]).as_deref(),
            Some("cfg-key")
        );
        unsafe { std::env::remove_var("TAVILY_API_KEY") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("TAVILY_API_KEY", v) };
        }
    }

    #[test]
    fn config_endpoint_overrides_env() {
        let prev = std::env::var("SEARXNG_URL").ok();
        unsafe { std::env::set_var("SEARXNG_URL", "http://env.example") };
        let cfg = WebSearchBackendConfigRef {
            endpoint: Some("http://cfg.example/".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_endpoint(&cfg, &["SEARXNG_URL"]).as_deref(),
            Some("http://cfg.example")
        );
        unsafe { std::env::remove_var("SEARXNG_URL") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("SEARXNG_URL", v) };
        }
    }

    #[test]
    fn brave_free_alias_finds_brave_config() {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "brave-free".into(),
            WebSearchBackendConfigRef {
                api_key: Some("free-key".into()),
                ..Default::default()
            },
        );
        let cfg = lookup_backend_config(&map, "brave");
        assert_eq!(cfg.api_key.as_deref(), Some("free-key"));
    }

    #[test]
    fn dotenv_key_counts_as_edgecrab_home_enabled() {
        let _lock = crate::tools::web::search::test_isolation::web_config_test_lock();
        let dir = tempfile::TempDir::new().expect("tempdir");
        let prev = std::env::var("EDGECRAB_HOME").ok();
        unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };
        std::fs::write(
            dir.path().join(".env"),
            "FIRECRAWL_API_KEY=fc-from-dotenv\n",
        )
        .expect("write env");
        assert!(backend_is_enabled_in_edgecrab_home(
            "firecrawl",
            &WebSearchBackendConfigRef::default()
        ));
        unsafe { std::env::remove_var("EDGECRAB_HOME") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("EDGECRAB_HOME", v) };
        }
    }

    #[test]
    fn process_env_alone_does_not_count_for_auto_chain() {
        let _lock = crate::tools::web::search::test_isolation::web_config_test_lock();
        let _registry = crate::tools::web::search::registry::test_registry_lock();
        let dir = tempfile::TempDir::new().expect("tempdir");
        let prev_home = std::env::var("EDGECRAB_HOME").ok();
        let prev_fc = std::env::var("FIRECRAWL_API_KEY").ok();
        unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };
        unsafe { std::env::set_var("FIRECRAWL_API_KEY", "only-in-process") };
        assert!(!backend_is_enabled_in_edgecrab_home(
            "firecrawl",
            &WebSearchBackendConfigRef::default()
        ));
        unsafe { std::env::remove_var("FIRECRAWL_API_KEY") };
        unsafe { std::env::remove_var("EDGECRAB_HOME") };
        if let Some(v) = prev_fc {
            unsafe { std::env::set_var("FIRECRAWL_API_KEY", v) };
        }
        if let Some(v) = prev_home {
            unsafe { std::env::set_var("EDGECRAB_HOME", v) };
        }
    }
}
