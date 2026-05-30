//! Web search configuration — env, config.yaml, and ToolContext overrides.

use std::path::Path;

use crate::config_ref::{WebSearchBackendConfigRef, WebSearchConfigRef, resolve_edgecrab_home};
use crate::tools::web::search::backend_settings::{
    MAX_SEARCH_RESULTS, backend_is_configured, lookup_backend_config,
};
use crate::tools::web::search::error::SearchError;

/// Per-request search parameters passed to backends.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub max_results: usize,
    pub timeout_secs: u64,
    pub backend_override: Option<String>,
    /// Resolved credentials for the active backend in the chain (config → env).
    pub backend_config: WebSearchBackendConfigRef,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_results: 5,
            timeout_secs: 8,
            backend_override: None,
            backend_config: WebSearchBackendConfigRef::default(),
        }
    }
}

impl SearchOptions {
    pub fn max_results(&self) -> usize {
        self.max_results.clamp(1, MAX_SEARCH_RESULTS)
    }
}

/// Per-request extract parameters passed to extract-capable backends.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub timeout_secs: u64,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self { timeout_secs: 30 }
    }
}

impl ExtractOptions {
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs.max(1)
    }
}

/// Resolved chain order: primary first, then fallbacks.
#[derive(Debug, Clone)]
pub struct ResolvedChain {
    pub names: Vec<String>,
    pub config: WebSearchConfigRef,
}

impl ResolvedChain {
    /// Build chain from config + optional per-call override.
    ///
    /// Unconfigured paid backends are **removed** from multi-backend chains (never
    /// attempted). Explicit single-backend selection (tool arg, env, or `web:` override)
    /// returns an error when credentials are missing so the agent does not burn turns.
    pub fn resolve(
        cfg: &WebSearchConfigRef,
        override_backend: Option<&str>,
    ) -> Result<Self, SearchError> {
        if let Some(name) = override_backend
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| !v.is_empty() && *v != "auto")
        {
            let names = finalize_chain_names(vec![name], cfg, ChainSelection::ExplicitSingle)?;
            return Ok(Self {
                names,
                config: cfg.clone(),
            });
        }

        if override_backend.is_none()
            && let Some(env) = env_backend_override()
            .filter(|n| n != "ddgs")
        {
            let names = finalize_chain_names(vec![env], cfg, ChainSelection::ExplicitSingle)?;
            return Ok(Self {
                names,
                config: cfg.clone(),
            });
        }

        if override_backend.is_none()
            && env_backend_override().is_none()
            && let Some(name) =
                crate::tools::web::search::web_config::resolve_config_search_backend()
        {
            let names = finalize_chain_names(vec![name], cfg, ChainSelection::ExplicitSingle)?;
            return Ok(Self {
                names,
                config: cfg.clone(),
            });
        }

        let names =
            finalize_chain_names(build_config_chain(cfg), cfg, ChainSelection::ConfigChain)?;

        Ok(Self {
            names,
            config: cfg.clone(),
        })
    }

    pub fn backend_config(&self, name: &str) -> WebSearchBackendConfigRef {
        lookup_backend_config(&self.config.backends, name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainSelection {
    /// Per-call `backend` arg or `EDGECRAB_WEB_*` env override.
    ExplicitSingle,
    /// `web_search.primary` / fallbacks / legacy auto chain.
    ConfigChain,
}

/// Drop backends missing credentials; never attempt HTTP without keys.
fn finalize_chain_names(
    names: Vec<String>,
    cfg: &WebSearchConfigRef,
    selection: ChainSelection,
) -> Result<Vec<String>, SearchError> {
    if selection == ChainSelection::ExplicitSingle {
        let name = names
            .first()
            .cloned()
            .ok_or_else(|| SearchError::hard("web_search", "No web search backend selected."))?;

        // ddgs is the no-key fallback tier — never collapse the chain to ddgs-only.
        // Try ddgs first when explicitly requested, then configured paid backends.
        if name == "ddgs" {
            let mut chain = build_config_chain(cfg);
            chain.retain(|n| n != "ddgs");
            chain.insert(0, "ddgs".into());
            return finalize_chain_names(chain, cfg, ChainSelection::ConfigChain);
        }

        let bc = lookup_backend_config(&cfg.backends, &name);
        if backend_is_configured(&name, &bc) {
            return Ok(vec![name]);
        }
        tracing::warn!(
            backend = %name,
            "web_search: explicit backend not configured — falling back to configured chain"
        );
        return finalize_chain_names(auto_chain_for_cfg(cfg), cfg, ChainSelection::ConfigChain);
    }

    let configured: Vec<String> = names
        .into_iter()
        .filter(|n| backend_is_configured(n, &lookup_backend_config(&cfg.backends, n)))
        .collect();

    if configured.is_empty() {
        return Ok(vec!["ddgs".into()]);
    }

    if configured.iter().all(|n| n == "ddgs") {
        return Ok(configured);
    }

    if configured.iter().any(|n| n == "ddgs") {
        Ok(configured)
    } else {
        let mut out = configured;
        out.push("ddgs".into());
        Ok(out)
    }
}

/// Whether `web_search` should be exposed to the agent (ddgs is always reachable).
pub fn web_search_is_available(_cfg: &WebSearchConfigRef) -> bool {
    true
}

/// Backends from `web_search.primary` + `fallbacks` that have credentials configured.
pub fn filter_configured_backends(names: &[String], cfg: &WebSearchConfigRef) -> Vec<String> {
    names
        .iter()
        .filter(|n| backend_is_configured(n, &lookup_backend_config(&cfg.backends, n)))
        .cloned()
        .collect()
}

fn build_config_chain(cfg: &WebSearchConfigRef) -> Vec<String> {
    let mut yaml_names = Vec::new();
    let primary = cfg.primary.trim().to_ascii_lowercase();
    if !primary.is_empty() && primary != "auto" {
        yaml_names.push(primary);
    }
    for fb in &cfg.fallbacks {
        let fb = fb.trim().to_ascii_lowercase();
        if !fb.is_empty() && !yaml_names.contains(&fb) {
            yaml_names.push(fb);
        }
    }

    if yaml_names.is_empty() {
        let names: Vec<String> = FREE_TIER_CHAIN
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let configured = filter_configured_backends(&names, cfg);
        return if configured.is_empty() {
            vec!["ddgs".into()]
        } else {
            configured
        };
    }

    let yaml_configured = filter_configured_backends(&yaml_names, cfg);
    if yaml_configured.is_empty() {
        vec!["ddgs".into()]
    } else {
        yaml_configured
    }
}

fn env_backend_override() -> Option<String> {
    ["EDGECRAB_WEB_SEARCH_BACKEND", "EDGECRAB_WEB_BACKEND"]
        .into_iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .map(|v| v.trim().to_ascii_lowercase())
                .filter(|v| !v.is_empty() && v != "auto")
        })
}

/// Clears env backend overrides for the duration of a test (restores on drop).
#[cfg(test)]
pub(crate) struct EnvBackendGuard {
    backend: Option<String>,
    web: Option<String>,
}

#[cfg(test)]
impl EnvBackendGuard {
    pub(crate) fn isolate() -> Self {
        let backend = std::env::var("EDGECRAB_WEB_SEARCH_BACKEND").ok();
        let web = std::env::var("EDGECRAB_WEB_BACKEND").ok();
        unsafe {
            std::env::remove_var("EDGECRAB_WEB_SEARCH_BACKEND");
            std::env::remove_var("EDGECRAB_WEB_BACKEND");
        }
        Self { backend, web }
    }
}

#[cfg(test)]
impl Drop for EnvBackendGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("EDGECRAB_WEB_SEARCH_BACKEND");
            std::env::remove_var("EDGECRAB_WEB_BACKEND");
        }
        if let Some(v) = &self.backend {
            unsafe { std::env::set_var("EDGECRAB_WEB_SEARCH_BACKEND", v) };
        }
        if let Some(v) = &self.web {
            unsafe { std::env::set_var("EDGECRAB_WEB_BACKEND", v) };
        }
    }
}

/// Free-tier backends used when `web_search.primary` / `fallbacks` are unset.
pub const FREE_TIER_CHAIN: &[&str] = &["searxng", "brave", "ddgs"];

/// Default chain when no explicit primary/fallbacks: paid APIs → free tiers.
/// Hermes legacy preference order when primary/fallbacks are empty.
pub const LEGACY_AUTO_CHAIN: &[&str] = &[
    "firecrawl",
    "parallel",
    "tavily",
    "exa",
    "searxng",
    "brave",
    "ddgs",
];

fn auto_chain_for_cfg(cfg: &WebSearchConfigRef) -> Vec<String> {
    let mut chain = Vec::new();
    for name in LEGACY_AUTO_CHAIN {
        let bc = lookup_backend_config(&cfg.backends, name);
        if super::backend_settings::backend_is_enabled_in_edgecrab_home(name, &bc) {
            chain.push((*name).into());
        }
    }
    if chain.is_empty() {
        chain.push("ddgs".into());
    }
    chain
}

fn backend_ready(name: &str, disk: &WebSearchConfigRef) -> bool {
    let cfg = lookup_backend_config(&disk.backends, name);
    super::backend_settings::backend_is_enabled_in_edgecrab_home(name, &cfg)
}

/// Empty config when `web_search:` section is absent from config.yaml.
pub fn empty_web_search_config() -> WebSearchConfigRef {
    WebSearchConfigRef {
        primary: String::new(),
        fallbacks: Vec::new(),
        timeout_secs: 8,
        backends: Default::default(),
    }
}

/// Load `web_search` section from `~/.edgecrab/config.yaml` (tools crate has no core dep).
pub fn load_web_search_config_from_disk() -> WebSearchConfigRef {
    let path = resolve_edgecrab_home().join("config.yaml");
    load_web_search_config_from_path(&path).unwrap_or_else(empty_web_search_config)
}

pub fn load_web_search_config_from_path(path: &Path) -> Option<WebSearchConfigRef> {
    let content = std::fs::read_to_string(path).ok()?;
    let raw: serde_yml::Value = serde_yml::from_str(&content).ok()?;
    let section = raw.get("web_search")?;
    serde_yml::from_value(section.clone()).ok()
}

/// Partial update for `web_search:` primary / fallback chain (`config.yaml`).
#[derive(Debug, Clone, Default)]
pub struct WebSearchChainUpdate {
    pub primary: Option<String>,
    pub fallbacks: Option<Vec<String>>,
    pub timeout_secs: Option<u64>,
}

fn normalize_chain_backend(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn dedupe_chain_backends(primary: &str, fallbacks: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let primary = normalize_chain_backend(primary);
    if !primary.is_empty() && primary != "auto" {
        out.push(primary);
    }
    for fb in fallbacks {
        let fb = normalize_chain_backend(fb);
        if fb.is_empty() || fb == "auto" || out.contains(&fb) {
            continue;
        }
        out.push(fb);
    }
    out
}

/// Human-readable chain summary — reflects the chain that will actually run.
pub fn format_search_chain_summary(cfg: &WebSearchConfigRef) -> String {
    let timeout = cfg.timeout_secs.max(1);
    match ResolvedChain::resolve(cfg, None) {
        Ok(resolved) if !resolved.names.is_empty() => {
            format!("{} ({timeout}s timeout)", resolved.names.join(" → "))
        }
        _ => format!("ddgs ({timeout}s timeout)"),
    }
}

/// Merge primary / fallbacks / timeout into `web_search:` (creates section if missing).
pub fn persist_web_search_chain_in_config(
    config_path: &Path,
    update: &WebSearchChainUpdate,
) -> Result<(), std::io::Error> {
    let content = if config_path.exists() {
        std::fs::read_to_string(config_path)?
    } else {
        String::new()
    };
    let mut raw: serde_yml::Value = if content.trim().is_empty() {
        serde_yml::Mapping::new().into()
    } else {
        serde_yml::from_str(&content).map_err(std::io::Error::other)?
    };
    let root = raw
        .as_mapping_mut()
        .ok_or_else(|| std::io::Error::other("config root must be a mapping"))?;

    let mut section = root
        .get(serde_yml::Value::String("web_search".into()))
        .and_then(|v| v.as_mapping())
        .cloned()
        .unwrap_or_default();

    if let Some(ref primary) = update.primary {
        section.insert(
            serde_yml::Value::String("primary".into()),
            serde_yml::Value::String(normalize_chain_backend(primary)),
        );
    }
    if let Some(ref fallbacks) = update.fallbacks {
        let items: Vec<serde_yml::Value> = fallbacks
            .iter()
            .map(|fb| serde_yml::Value::String(normalize_chain_backend(fb)))
            .collect();
        section.insert(
            serde_yml::Value::String("fallbacks".into()),
            serde_yml::Value::Sequence(items),
        );
    }
    if let Some(timeout) = update.timeout_secs {
        section.insert(
            serde_yml::Value::String("timeout_secs".into()),
            serde_yml::Value::Number(timeout.into()),
        );
    }

    root.insert(
        serde_yml::Value::String("web_search".into()),
        serde_yml::Value::Mapping(section),
    );

    let serialized = serde_yml::to_string(&raw).map_err(std::io::Error::other)?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(config_path, serialized)
}

/// Reset `web_search.primary` / `fallbacks` to auto (empty) — keeps per-backend keys.
pub fn clear_web_search_chain_in_config(config_path: &Path) -> Result<(), std::io::Error> {
    persist_web_search_chain_in_config(
        config_path,
        &WebSearchChainUpdate {
            primary: Some(String::new()),
            fallbacks: Some(Vec::new()),
            timeout_secs: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_isolation::{EdgecrabHomeGuard, web_config_test_lock};
    use super::*;

    #[test]
    fn auto_chain_includes_ddgs_terminal_fallback() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(None);
        let chain = auto_chain_for_cfg(&WebSearchConfigRef::default());
        assert_eq!(chain.last().map(String::as_str), Some("ddgs"));
    }

    #[test]
    fn legacy_auto_chain_order_matches_hermes() {
        assert_eq!(
            LEGACY_AUTO_CHAIN,
            &[
                "firecrawl",
                "parallel",
                "tavily",
                "exa",
                "searxng",
                "brave",
                "ddgs",
            ]
        );
    }

    #[test]
    fn persist_web_search_chain_writes_yaml() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("config.yaml");
        persist_web_search_chain_in_config(
            &path,
            &WebSearchChainUpdate {
                primary: Some("searxng".into()),
                fallbacks: Some(vec!["brave".into(), "ddgs".into()]),
                timeout_secs: Some(12),
            },
        )
        .expect("persist");
        let cfg = load_web_search_config_from_path(&path).expect("parse");
        assert_eq!(cfg.primary, "searxng");
        assert_eq!(cfg.fallbacks, vec!["brave", "ddgs"]);
        assert_eq!(cfg.timeout_secs, 12);
        assert_eq!(
            format_search_chain_summary(&cfg),
            "ddgs (12s timeout)"
        );
    }

    #[test]
    fn env_only_paid_key_does_not_auto_upgrade_chain() {
        let _env = EnvBackendGuard::isolate();
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(None);
        let prev = std::env::var("FIRECRAWL_API_KEY").ok();
        unsafe { std::env::set_var("FIRECRAWL_API_KEY", "test-firecrawl-key") };
        unsafe { std::env::remove_var("SEARXNG_URL") };
        let cfg = empty_web_search_config();
        let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
        assert_eq!(resolved.names, vec!["ddgs"]);
        unsafe { std::env::remove_var("FIRECRAWL_API_KEY") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("FIRECRAWL_API_KEY", v) };
        }
    }

    #[test]
    fn empty_yaml_uses_free_tier_chain_when_searxng_configured() {
        let _env = EnvBackendGuard::isolate();
        let _lock = web_config_test_lock();
        let prev = std::env::var("SEARXNG_URL").ok();
        unsafe { std::env::set_var("SEARXNG_URL", "http://127.0.0.1:8888") };
        let cfg = empty_web_search_config();
        let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
        assert_eq!(resolved.names, vec!["searxng", "ddgs"]);
        unsafe { std::env::remove_var("SEARXNG_URL") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("SEARXNG_URL", v) };
        }
    }

    #[test]
    fn clear_web_search_chain_resets_primary_and_fallbacks() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("config.yaml");
        persist_web_search_chain_in_config(
            &path,
            &WebSearchChainUpdate {
                primary: Some("brave".into()),
                fallbacks: Some(vec!["ddgs".into()]),
                timeout_secs: None,
            },
        )
        .expect("persist");
        clear_web_search_chain_in_config(&path).expect("clear");
        let cfg = load_web_search_config_from_path(&path).expect("parse");
        assert!(cfg.primary.is_empty());
        assert!(cfg.fallbacks.is_empty());
    }

    #[test]
    fn explicit_override_is_single_backend() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(None);
        let _env = EnvBackendGuard::isolate();
        let prev = std::env::var("BRAVE_API_KEY").ok();
        unsafe { std::env::set_var("BRAVE_API_KEY", "test-brave-key") };
        let cfg = WebSearchConfigRef::default();
        let resolved = ResolvedChain::resolve(&cfg, Some("brave")).expect("resolve");
        assert_eq!(resolved.names, vec!["brave"]);
        unsafe { std::env::remove_var("BRAVE_API_KEY") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("BRAVE_API_KEY", v) };
        }
    }

    #[test]
    fn env_backend_override_wins_over_config() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(None);
        let _env = EnvBackendGuard::isolate();
        unsafe { std::env::set_var("EDGECRAB_WEB_SEARCH_BACKEND", "tavily") };
        unsafe { std::env::set_var("TAVILY_API_KEY", "test-tavily-key") };
        let cfg = WebSearchConfigRef {
            primary: "searxng".into(),
            fallbacks: vec!["ddgs".into()],
            ..Default::default()
        };
        let resolved = ResolvedChain::resolve(&cfg, None).expect("resolve");
        assert_eq!(resolved.names, vec!["tavily"]);
        unsafe { std::env::remove_var("TAVILY_API_KEY") };
    }

    #[test]
    fn auto_override_uses_full_chain() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(None);
        let _env = EnvBackendGuard::isolate();
        let prev = std::env::var("SEARXNG_URL").ok();
        unsafe { std::env::set_var("SEARXNG_URL", "http://searx.example") };
        let cfg = WebSearchConfigRef {
            primary: "searxng".into(),
            fallbacks: vec!["ddgs".into()],
            ..Default::default()
        };
        let resolved = ResolvedChain::resolve(&cfg, Some("auto")).expect("resolve");
        assert_eq!(resolved.names, vec!["searxng", "ddgs"]);
        unsafe { std::env::remove_var("SEARXNG_URL") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("SEARXNG_URL", v) };
        }
    }
}
