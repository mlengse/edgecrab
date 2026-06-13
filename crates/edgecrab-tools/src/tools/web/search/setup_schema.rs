//! Hermes-aligned setup picker metadata for web search/extract providers.
//!
//! Shape matches `WebSearchProvider.get_setup_schema()`:
//! `{ name, badge, tag, env_vars: [{ key, prompt, url? }] }`

use serde::Serialize;

/// Env var prompt row for setup wizards (`hermes tools` picker).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SetupEnvVar {
    pub key: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SetupEnvVar {
    pub fn new(key: impl Into<String>, prompt: impl Into<String>, url: Option<&str>) -> Self {
        Self {
            key: key.into(),
            prompt: prompt.into(),
            url: url.map(str::to_string),
        }
    }
}

/// Provider row metadata for setup / doctor tooling.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SetupSchema {
    pub name: String,
    pub badge: String,
    pub tag: String,
    pub env_vars: Vec<SetupEnvVar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_setup: Option<String>,
}

impl SetupSchema {
    pub fn minimal(display_name: impl Into<String>) -> Self {
        let name = display_name.into();
        Self {
            name: name.clone(),
            badge: String::new(),
            tag: String::new(),
            env_vars: Vec::new(),
            post_setup: None,
        }
    }
}

/// Built-in setup schema for a registered backend name.
pub fn setup_schema_for_backend(name: &str) -> SetupSchema {
    use crate::tools::web::search::backend_settings::normalize_backend_name;
    match normalize_backend_name(name).as_str() {
        "searxng" => SetupSchema {
            name: "SearXNG".into(),
            badge: "free · self-hosted".into(),
            tag: "Privacy-respecting metasearch. Set web_search.backends.searxng.endpoint or SEARXNG_URL.".into(),
            env_vars: vec![SetupEnvVar::new(
                "SEARXNG_URL",
                "SearXNG instance URL (e.g. http://localhost:8080)",
                Some("https://searx.space/"),
            )],
            post_setup: None,
        },
        "brave" => SetupSchema {
            name: "Brave Search (Free)".into(),
            badge: "free".into(),
            tag: "Free-tier API key — search only.".into(),
            env_vars: vec![SetupEnvVar::new(
                "BRAVE_SEARCH_API_KEY",
                "Brave Search API key (free tier; BRAVE_API_KEY also accepted)",
                Some("https://brave.com/search/api/"),
            )],
            post_setup: None,
        },
        "ddgs" => SetupSchema {
            name: "DuckDuckGo (ddgs)".into(),
            badge: "free · no key · search only".into(),
            tag: "Native Rust metasearch (Bing). Env: DDGS_REGION, DDGS_PROXY, DDGS_IMPERSONATE, DDGS_IMPERSONATE_OS.".into(),
            env_vars: vec![
                SetupEnvVar::new(
                    "DDGS_REGION",
                    "Locale/region — e.g. us-en, fr-fr, de-de",
                    None,
                ),
                SetupEnvVar::new(
                    "DDGS_PROXY",
                    "Proxy for metasearch (optional; HTTPS_PROXY also works)",
                    None,
                ),
                SetupEnvVar::new(
                    "DDGS_BACKEND",
                    "Engine: auto (default), bing, html, or lite",
                    None,
                ),
            ],
            post_setup: None,
        },
        "firecrawl" => SetupSchema {
            name: "Firecrawl".into(),
            badge: "paid · search + extract + crawl".into(),
            tag: "Premium search, scrape, and crawl via Firecrawl API.".into(),
            env_vars: vec![SetupEnvVar::new(
                "FIRECRAWL_API_KEY",
                "Firecrawl API key",
                Some("https://firecrawl.dev/"),
            )],
            post_setup: None,
        },
        "tavily" => SetupSchema {
            name: "Tavily".into(),
            badge: "paid · search + extract + crawl".into(),
            tag: "AI-native search and extract (~1000 free searches/month).".into(),
            env_vars: vec![SetupEnvVar::new(
                "TAVILY_API_KEY",
                "Tavily API key",
                Some("https://tavily.com/"),
            )],
            post_setup: None,
        },
        "exa" => SetupSchema {
            name: "Exa".into(),
            badge: "paid · search + extract".into(),
            tag: "Neural search and contents API.".into(),
            env_vars: vec![SetupEnvVar::new(
                "EXA_API_KEY",
                "Exa API key",
                Some("https://exa.ai/"),
            )],
            post_setup: None,
        },
        "parallel" => SetupSchema {
            name: "Parallel".into(),
            badge: "paid · search + extract".into(),
            tag: "Parallel web search and extract API.".into(),
            env_vars: vec![SetupEnvVar::new(
                "PARALLEL_API_KEY",
                "Parallel API key",
                Some("https://parallel.ai/"),
            )],
            post_setup: None,
        },
        "xai" => SetupSchema {
            name: "xAI Grok".into(),
            badge: "paid · search only".into(),
            tag: "Agentic web search via xAI API (search only).".into(),
            env_vars: vec![SetupEnvVar::new(
                "XAI_API_KEY",
                "xAI API key",
                Some("https://x.ai/"),
            )],
            post_setup: None,
        },
        other => SetupSchema::minimal(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILTIN: &[&str] = &[
        "searxng",
        "brave",
        "ddgs",
        "firecrawl",
        "tavily",
        "exa",
        "parallel",
        "xai",
    ];

    #[test]
    fn each_builtin_has_picker_schema() {
        for name in BUILTIN {
            let schema = setup_schema_for_backend(name);
            assert!(!schema.name.is_empty(), "{name} name");
            assert!(
                schema.name.len() >= 2,
                "{name} should have human-readable name"
            );
            // Hermes contract: env_vars key always present (may be empty for ddgs).
            let _ = &schema.env_vars;
        }
    }

    #[test]
    fn ddgs_documents_optional_env_and_brave_has_key() {
        let ddgs = setup_schema_for_backend("ddgs");
        assert!(ddgs.env_vars.len() >= 2);
        assert!(ddgs.env_vars.iter().any(|e| e.key == "DDGS_REGION"));
        assert!(ddgs.post_setup.is_none());
        assert_eq!(
            setup_schema_for_backend("brave").env_vars[0].key,
            "BRAVE_SEARCH_API_KEY"
        );
    }

    #[test]
    fn schema_serializes_to_json_object() {
        let schema = setup_schema_for_backend("searxng");
        let value = serde_json::to_value(&schema).expect("serialize");
        assert!(value.get("name").is_some());
        assert!(value.get("env_vars").and_then(|v| v.as_array()).is_some());
    }
}
