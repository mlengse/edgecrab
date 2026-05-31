//! Web provider status for `edgecrab doctor` and setup flows (Hermes tools picker parity).
//!
//! Aggregates registry backends, setup schemas, and credential probes into one report.

use serde::Serialize;

use super::backend_settings::{backend_is_configured, lookup_backend_config};
use super::config::{format_search_chain_summary, load_web_search_config_from_disk};
use super::provider_capabilities;
use super::registry::list_web_search_backends;
use super::setup_schema::SetupSchema;
use super::web_config::{
    config_extract_backend_choice, config_search_backend_choice, resolve_config_extract_backend,
    resolve_config_search_backend,
};
use crate::config_ref::WebSearchBackendConfigRef;

/// One registered web provider row (mirrors Hermes picker + doctor hints).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WebProviderStatus {
    pub id: String,
    pub display_name: String,
    pub badge: String,
    pub configured: bool,
    pub available: bool,
    pub supports_search: bool,
    pub supports_extract: bool,
    pub supports_crawl: bool,
    /// Env var keys from setup schema that are still unset (when not configured).
    pub missing_env: Vec<String>,
}

/// Aggregate report consumed by CLI doctor / setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDiagnosticsReport {
    pub providers: Vec<WebProviderStatus>,
    /// At least one search backend can run (includes keyless ddgs).
    pub search_ready: bool,
    /// Paid extract API configured, or native extract always available.
    pub paid_extract_configured: bool,
    pub configured_search_override: Option<String>,
    pub configured_extract_override: Option<String>,
    pub resolved_search_backend: Option<String>,
    pub resolved_extract_backend: Option<String>,
    pub search_chain_primary: Option<String>,
    pub search_chain_fallbacks: Vec<String>,
    pub search_chain_timeout_secs: u64,
    pub search_chain_summary: String,
}

fn env_var_set(key: &str) -> bool {
    std::env::var(key)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

fn missing_env_keys(
    name: &str,
    schema: &SetupSchema,
    cfg: &WebSearchBackendConfigRef,
) -> Vec<String> {
    if backend_is_configured(name, cfg) {
        return Vec::new();
    }
    schema
        .env_vars
        .iter()
        .filter(|ev| !env_var_set(&ev.key))
        .map(|ev| ev.key.clone())
        .collect()
}

/// Build a full diagnostics snapshot from the live registry + on-disk config.
pub fn collect_web_diagnostics() -> WebDiagnosticsReport {
    let disk = load_web_search_config_from_disk();
    let mut providers = Vec::new();
    let mut search_ready = false;
    let mut paid_extract_configured = false;

    for backend in list_web_search_backends() {
        let id = backend.name().to_string();
        let schema = backend.setup_schema();
        let cfg = lookup_backend_config(&disk.backends, &id);
        let configured = backend_is_configured(&id, &cfg);
        let available = backend.is_available();
        let supports_search = provider_capabilities::supports_search(&id);
        let supports_extract = provider_capabilities::supports_extract(&id);
        let supports_crawl = provider_capabilities::supports_crawl(&id);

        if supports_search && (id == "ddgs" || (configured && available)) {
            search_ready = true;
        }
        if supports_extract && configured && available && id != "native" && id != "browser" {
            paid_extract_configured = true;
        }

        providers.push(WebProviderStatus {
            id: id.clone(),
            display_name: schema.name.clone(),
            badge: schema.badge.clone(),
            configured,
            available,
            supports_search,
            supports_extract,
            supports_crawl,
            missing_env: missing_env_keys(&id, &schema, &cfg),
        });
    }

    let configured_search_override = config_search_backend_choice().filter(|n| n != "ddgs");
    let configured_extract_override = config_extract_backend_choice();
    let resolved_search_backend = resolve_config_search_backend();
    let resolved_extract_backend = resolve_config_extract_backend();
    let search_chain_primary = disk.primary.trim().to_ascii_lowercase();
    let search_chain_primary = if search_chain_primary.is_empty() || search_chain_primary == "auto"
    {
        None
    } else {
        Some(search_chain_primary)
    };
    let search_chain_fallbacks = disk
        .fallbacks
        .iter()
        .map(|fb| fb.trim().to_ascii_lowercase())
        .filter(|fb| !fb.is_empty() && fb != "auto")
        .collect::<Vec<_>>();
    let search_chain_timeout_secs = disk.timeout_secs;
    let search_chain_summary = format_search_chain_summary(&disk);

    WebDiagnosticsReport {
        providers,
        search_ready,
        paid_extract_configured,
        configured_search_override,
        configured_extract_override,
        resolved_search_backend,
        resolved_extract_backend,
        search_chain_primary,
        search_chain_fallbacks,
        search_chain_timeout_secs,
        search_chain_summary,
    }
}

/// One-line summary for doctor output.
pub fn format_search_doctor_detail(report: &WebDiagnosticsReport) -> String {
    let configured: Vec<_> = report
        .providers
        .iter()
        .filter(|p| p.supports_search && p.configured && p.available)
        .map(|p| p.id.as_str())
        .collect();

    if let Some(ref name) = report.resolved_search_backend {
        return format!("configured → {name}");
    }
    if configured.is_empty() {
        if report.search_ready {
            "ddgs HTML fallback (no API keys)".into()
        } else {
            "no search backends configured — set BRAVE_SEARCH_API_KEY, SEARXNG_URL, or run edgecrab setup tools".into()
        }
    } else {
        format!("ready: {}", configured.join(", "))
    }
}

/// One-line summary for extract doctor output.
pub fn format_extract_doctor_detail(report: &WebDiagnosticsReport) -> String {
    if let Some(ref name) = report.resolved_extract_backend {
        return format!("configured → {name}");
    }
    if report.paid_extract_configured {
        let names: Vec<_> = report
            .providers
            .iter()
            .filter(|p| p.supports_extract && p.configured && p.available)
            .map(|p| p.id.as_str())
            .collect();
        format!("paid APIs: {} + native wreq fallback", names.join(", "))
    } else {
        "native wreq/EdgeParse fallback (no paid extract API keys)".into()
    }
}

/// Hermes-style picker rows (JSON-serializable) for setup tooling.
pub fn web_provider_picker_rows() -> Vec<serde_json::Value> {
    list_web_search_backends()
        .into_iter()
        .map(|b| {
            let id = b.name().to_string();
            let schema = b.setup_schema();
            let cfg = lookup_backend_config(&load_web_search_config_from_disk().backends, &id);
            serde_json::json!({
                "id": id,
                "name": schema.name,
                "badge": schema.badge,
                "tag": schema.tag,
                "env_vars": schema.env_vars,
                "configured": backend_is_configured(b.name(), &cfg),
                "available": b.is_available(),
                "supports_search": provider_capabilities::supports_search(b.name()),
                "supports_extract": provider_capabilities::supports_extract(b.name()),
                "supports_crawl": provider_capabilities::supports_crawl(b.name()),
            })
        })
        .collect()
}

/// Compact capability label for picker rows (e.g. `S+E+C`).
pub fn capability_label(
    supports_search: bool,
    supports_extract: bool,
    supports_crawl: bool,
) -> String {
    let mut parts = Vec::new();
    if supports_search {
        parts.push('S');
    }
    if supports_extract {
        parts.push('E');
    }
    if supports_crawl {
        parts.push('C');
    }
    if parts.is_empty() {
        "—".into()
    } else {
        parts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("+")
    }
}

/// Rich status report for TUI overlay and setup wizard header.
pub fn format_web_setup_report(report: &WebDiagnosticsReport) -> String {
    let mut out = String::new();
    out.push_str("Web Search & Extract — Status\n");
    out.push_str("────────────────────────────────\n");
    out.push_str(&format!(
        "Search:  {}\n",
        format_search_doctor_detail(report)
    ));
    out.push_str(&format!(
        "Extract: {}\n",
        format_extract_doctor_detail(report)
    ));
    if let Some(ref s) = report.configured_search_override {
        out.push_str(&format!(
            "Config:  web.search_backend={s} (overrides chain)\n"
        ));
    } else if let Some(ref e) = report.configured_extract_override {
        out.push_str(&format!("Config:  web.extract_backend={e}\n"));
    } else if let Some(ref b) = report.resolved_search_backend {
        out.push_str(&format!("Config:  web.backend={b} (overrides chain)\n"));
    } else {
        out.push_str("Config:  auto (fallback chain + native extract)\n");
    }
    out.push_str(&format!("Chain:   {}\n", report.search_chain_summary));
    out.push_str("\nProviders:\n");
    for p in &report.providers {
        let caps = capability_label(p.supports_search, p.supports_extract, p.supports_crawl);
        let state = if p.configured && p.available {
            "ready"
        } else if p.configured {
            "configured"
        } else if p.missing_env.is_empty() {
            "no key needed"
        } else {
            "needs key"
        };
        out.push_str(&format!(
            "  {:<12} [{:<3}] {:<12} {}\n",
            p.id, caps, state, p.display_name
        ));
    }
    out.push_str("\nCommands: edgecrab setup web  |  /web [status|setup|chain|doctor]\n");
    out
}

#[cfg(test)]
mod tests {
    use super::super::registry::{reset_registry_for_tests, test_registry_lock};
    use super::super::test_isolation::EdgecrabHomeGuard;
    use super::*;

    #[test]
    fn collect_includes_all_builtins() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let report = collect_web_diagnostics();
        assert!(report.providers.len() >= 8);
        assert!(report.search_ready, "ddgs should make search_ready true");
    }

    #[test]
    fn format_web_setup_report_lists_providers() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let report = collect_web_diagnostics();
        let text = format_web_setup_report(&report);
        assert!(text.contains("Providers:"));
        assert!(text.contains("firecrawl"));
        assert!(text.contains("Search:"));
    }

    #[test]
    fn capability_label_marks_search_only() {
        assert_eq!(capability_label(true, false, false), "S");
        assert_eq!(capability_label(true, true, true), "S+E+C");
    }

    #[test]
    fn brave_missing_env_when_unconfigured() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let prev = std::env::var("BRAVE_SEARCH_API_KEY").ok();
        let prev2 = std::env::var("BRAVE_API_KEY").ok();
        unsafe { std::env::remove_var("BRAVE_SEARCH_API_KEY") };
        unsafe { std::env::remove_var("BRAVE_API_KEY") };
        let report = collect_web_diagnostics();
        let brave = report
            .providers
            .iter()
            .find(|p| p.id == "brave")
            .expect("brave row");
        assert!(!brave.configured);
        assert!(!brave.missing_env.is_empty());
        if let Some(v) = prev {
            unsafe { std::env::set_var("BRAVE_SEARCH_API_KEY", v) };
        }
        if let Some(v) = prev2 {
            unsafe { std::env::set_var("BRAVE_API_KEY", v) };
        }
    }

    #[test]
    fn picker_rows_match_hermes_shape() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let rows = web_provider_picker_rows();
        assert!(rows.len() >= 8);
        for row in rows {
            assert!(row.get("name").is_some());
            assert!(row.get("env_vars").is_some());
            assert!(row.get("supports_search").is_some());
        }
    }

    #[test]
    fn resolved_override_from_web_section() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let _env = super::super::config::EnvBackendGuard::isolate();
        let _cfg_lock = super::super::test_isolation::web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(Some(
            r#"
web_search:
  backends:
    brave:
      api_key: test-brave
web:
  search_backend: brave
"#,
        ));
        let prev = std::env::var("BRAVE_API_KEY").ok();
        let prev2 = std::env::var("BRAVE_SEARCH_API_KEY").ok();
        unsafe { std::env::remove_var("BRAVE_API_KEY") };
        unsafe { std::env::remove_var("BRAVE_SEARCH_API_KEY") };
        let report = collect_web_diagnostics();
        assert_eq!(report.resolved_search_backend.as_deref(), Some("brave"));
        if let Some(v) = prev {
            unsafe { std::env::set_var("BRAVE_API_KEY", v) };
        }
        if let Some(v) = prev2 {
            unsafe { std::env::set_var("BRAVE_SEARCH_API_KEY", v) };
        }
    }
}
