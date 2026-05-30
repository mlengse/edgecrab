//! DDGS runtime settings — env overrides with sensible defaults (no API key).

use rand::seq::SliceRandom;

use crate::config_ref::WebSearchBackendConfigRef;

/// Metasearch engine (reverse-engineered from Python `ddgs` package).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdgsEngine {
    Bing,
    Html,
    Lite,
}

impl DdgsEngine {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bing => "Bing",
            Self::Html => "DuckDuckGo HTML",
            Self::Lite => "DuckDuckGo Lite",
        }
    }
}

/// Which engine(s) to use — `auto` shuffles Bing + DDG HTML + DDG lite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DdgsBackendMode {
    #[default]
    Auto,
    Bing,
    Html,
    Lite,
}

/// DuckDuckGo HTML search tunables (Hermes `ddgs` package exposes region via CLI; we use env).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdgsSettings {
    /// `kl` form field — locale/region (e.g. `us-en`, `fr-fr`).
    pub region: String,
    /// `kp` form field — `-1` strict, `-2` moderate (default), `1` off (HTML backend only).
    pub safe_search: String,
    /// Transient-error retries per engine before trying the next metasearch backend.
    pub max_retries: u32,
    /// Metasearch mode (`DDGS_BACKEND=auto|bing|html|lite`).
    pub backend_mode: DdgsBackendMode,
}

impl Default for DdgsSettings {
    fn default() -> Self {
        Self {
            region: "us-en".into(),
            safe_search: "-2".into(),
            max_retries: 2,
            backend_mode: DdgsBackendMode::Auto,
        }
    }
}

impl DdgsSettings {
    /// Resolve from `web_search.backends.ddgs` + env (`DDGS_REGION`, `DDGS_SAFESEARCH`, `DDGS_MAX_RETRIES`, `DDGS_BACKEND`).
    pub fn resolve(cfg: &WebSearchBackendConfigRef) -> Self {
        let mut settings = Self::default();

        if let Some(region) = cfg
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            settings.region = region.to_ascii_lowercase();
        }
        if let Some(region) = std::env::var("DDGS_REGION")
            .ok()
            .map(|r| r.trim().to_ascii_lowercase())
            .filter(|r| !r.is_empty())
        {
            settings.region = region;
        }

        if let Ok(kp) = std::env::var("DDGS_SAFESEARCH") {
            let kp = kp.trim();
            if matches!(kp, "-1" | "-2" | "1") {
                settings.safe_search = kp.to_string();
            }
        }

        if let Ok(n) = std::env::var("DDGS_MAX_RETRIES")
            && let Ok(parsed) = n.trim().parse::<u32>()
        {
            settings.max_retries = parsed.min(5);
        }

        if let Ok(mode) = std::env::var("DDGS_BACKEND") {
            settings.backend_mode = match mode.trim().to_ascii_lowercase().as_str() {
                "bing" => DdgsBackendMode::Bing,
                "html" => DdgsBackendMode::Html,
                "lite" => DdgsBackendMode::Lite,
                "auto" => DdgsBackendMode::Auto,
                _ => settings.backend_mode,
            };
        }

        settings
    }

    /// Engine try-order for this configuration (Python `ddgs.text(..., backend=...)` parity).
    pub fn engine_order(&self) -> Vec<DdgsEngine> {
        match self.backend_mode {
            DdgsBackendMode::Bing => vec![DdgsEngine::Bing],
            DdgsBackendMode::Html => vec![DdgsEngine::Html],
            DdgsBackendMode::Lite => vec![DdgsEngine::Lite],
            DdgsBackendMode::Auto => {
                // Bing is most reliable; shuffle DDG fallbacks (Python package default).
                let mut fallbacks = [DdgsEngine::Html, DdgsEngine::Lite];
                fallbacks.shuffle(&mut rand::rng());
                vec![DdgsEngine::Bing, fallbacks[0], fallbacks[1]]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::web::search::test_isolation::web_config_test_lock;

    #[test]
    fn defaults_match_ddg_html_form() {
        let _lock = web_config_test_lock();
        let s = DdgsSettings::default();
        assert_eq!(s.region, "us-en");
        assert_eq!(s.safe_search, "-2");
        assert_eq!(s.max_retries, 2);
        assert_eq!(s.backend_mode, DdgsBackendMode::Auto);
    }

    #[test]
    fn env_overrides_region_and_retries() {
        let _lock = web_config_test_lock();
        let prev_r = std::env::var("DDGS_REGION").ok();
        let prev_n = std::env::var("DDGS_MAX_RETRIES").ok();
        unsafe { std::env::set_var("DDGS_REGION", "fr-fr") };
        unsafe { std::env::set_var("DDGS_MAX_RETRIES", "4") };
        let s = DdgsSettings::resolve(&WebSearchBackendConfigRef::default());
        assert_eq!(s.region, "fr-fr");
        assert_eq!(s.max_retries, 4);
        unsafe { std::env::remove_var("DDGS_REGION") };
        unsafe { std::env::remove_var("DDGS_MAX_RETRIES") };
        if let Some(v) = prev_r {
            unsafe { std::env::set_var("DDGS_REGION", v) };
        }
        if let Some(v) = prev_n {
            unsafe { std::env::set_var("DDGS_MAX_RETRIES", v) };
        }
    }

    #[test]
    fn config_endpoint_sets_region_when_env_unset() {
        let _lock = web_config_test_lock();
        let prev = std::env::var("DDGS_REGION").ok();
        unsafe { std::env::remove_var("DDGS_REGION") };
        let s = DdgsSettings::resolve(&WebSearchBackendConfigRef {
            endpoint: Some("de-de".into()),
            ..Default::default()
        });
        assert_eq!(s.region, "de-de");
        unsafe { std::env::remove_var("DDGS_REGION") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("DDGS_REGION", v) };
        }
    }

    #[test]
    fn auto_engine_order_prefers_bing_first() {
        let _lock = web_config_test_lock();
        let order = DdgsSettings::default().engine_order();
        assert_eq!(order[0], DdgsEngine::Bing);
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn backend_mode_env_selects_single_engine() {
        let _lock = web_config_test_lock();
        let prev = std::env::var("DDGS_BACKEND").ok();
        unsafe { std::env::set_var("DDGS_BACKEND", "html") };
        let s = DdgsSettings::resolve(&WebSearchBackendConfigRef::default());
        assert_eq!(s.engine_order(), vec![DdgsEngine::Html]);
        unsafe { std::env::remove_var("DDGS_BACKEND") };
        if let Some(v) = prev {
            unsafe { std::env::set_var("DDGS_BACKEND", v) };
        }
    }
}
