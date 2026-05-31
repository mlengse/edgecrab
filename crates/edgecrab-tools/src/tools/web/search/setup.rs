//! Shared web setup logic — single source for `/web` TUI and `edgecrab setup web`.
//!
//! **SRP:** chain editing, picker labels, and persistence live here; CLI/TUI only render.

use std::path::Path;

use serde_json::Value;

use super::config::{
    WebSearchChainUpdate, clear_web_search_chain_in_config, format_search_chain_summary,
    load_web_search_config_from_disk, load_web_search_config_from_path,
    persist_web_search_chain_in_config, ResolvedChain,
};
use super::provider_diagnostics::{capability_label, web_provider_picker_rows};
use super::web_config::{clear_web_search_section_overrides, clear_web_section_overrides};
use crate::config_ref::resolve_edgecrab_home;

/// Errors from in-memory chain edits (before persist).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainEditError {
    AlreadyInChain,
    NotInChainEligibleList,
    ChainTooShort,
    InvalidIndex,
}

impl ChainEditError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::AlreadyInChain => "Provider is already in the chain",
            Self::NotInChainEligibleList => "Provider cannot be added to the search chain",
            Self::ChainTooShort => "Keep at least one provider — reset to auto instead",
            Self::InvalidIndex => "Invalid chain index",
        }
    }
}

/// Provider rows + derived chain-eligible backend ids.
#[derive(Debug, Clone)]
pub struct WebPickerCatalog {
    pub rows: Vec<Value>,
    /// Search backends that may appear in the chain (`ddgs` or configured).
    pub chain_eligible_ids: Vec<String>,
}

impl WebPickerCatalog {
    pub fn load() -> Self {
        let rows = web_provider_picker_rows();
        let chain_eligible_ids = chain_eligible_search_ids(&rows);
        Self {
            rows,
            chain_eligible_ids,
        }
    }

    pub fn row_for(&self, id: &str) -> Option<&Value> {
        self.rows.iter().find(|r| r["id"].as_str() == Some(id))
    }

    pub fn picker_label(&self, id: &str) -> String {
        self.row_for(id)
            .map(format_picker_label)
            .unwrap_or_else(|| id.to_string())
    }
}

/// In-memory search priority chain (primary = index 0).
#[derive(Debug, Clone)]
pub struct WebChainEditor {
    pub catalog: WebPickerCatalog,
    pub order: Vec<String>,
    /// No explicit `web_search.primary` in config — show resolved preview.
    pub is_auto: bool,
}

impl WebChainEditor {
    pub fn load_from_disk() -> Self {
        let catalog = WebPickerCatalog::load();
        let disk = load_web_search_config_from_disk();
        let is_auto = disk.primary.trim().is_empty();
        let order = if is_auto {
            ResolvedChain::resolve(&disk, None)
                .ok()
                .map(|r| r.names)
                .filter(|names| !names.is_empty())
                .unwrap_or_else(|| vec!["ddgs".into()])
        } else {
            order_from_primary_and_fallbacks(&disk.primary, &disk.fallbacks)
        };
        Self {
            catalog,
            order,
            is_auto,
        }
    }

    pub fn reload(&mut self) {
        *self = Self::load_from_disk();
    }

    pub fn available_ids(&self) -> Vec<String> {
        self.catalog
            .chain_eligible_ids
            .iter()
            .filter(|id| !self.order.contains(id))
            .cloned()
            .collect()
    }

    pub fn summary_arrow(&self) -> String {
        if self.order.is_empty() {
            "ddgs".into()
        } else {
            self.order.join(" → ")
        }
    }

    pub fn mode_label(&self) -> &'static str {
        if self.is_auto {
            "auto"
        } else {
            "custom"
        }
    }

    pub fn move_item(&mut self, index: usize, delta: i32) -> Result<usize, ChainEditError> {
        if index >= self.order.len() {
            return Err(ChainEditError::InvalidIndex);
        }
        let new_index = index as i32 + delta;
        if new_index < 0 || new_index as usize >= self.order.len() {
            return Err(ChainEditError::InvalidIndex);
        }
        let new_index = new_index as usize;
        self.order.swap(index, new_index);
        self.is_auto = false;
        Ok(new_index)
    }

    pub fn add_backend(&mut self, id: &str) -> Result<(), ChainEditError> {
        if self.order.iter().any(|x| x == id) {
            return Err(ChainEditError::AlreadyInChain);
        }
        if !self.catalog.chain_eligible_ids.iter().any(|x| x == id) {
            return Err(ChainEditError::NotInChainEligibleList);
        }
        self.order.push(id.to_string());
        self.is_auto = false;
        Ok(())
    }

    pub fn remove_at(&mut self, index: usize) -> Result<String, ChainEditError> {
        if self.order.len() <= 1 {
            return Err(ChainEditError::ChainTooShort);
        }
        if index >= self.order.len() {
            return Err(ChainEditError::InvalidIndex);
        }
        let removed = self.order.remove(index);
        self.is_auto = false;
        Ok(removed)
    }

    pub fn persist(&self, config_path: &Path) -> std::io::Result<()> {
        persist_search_chain_order(config_path, &self.order)
    }

    /// True when `web.search_backend` / `web.backend` would override the chain at runtime.
    pub fn search_section_override_active(&self) -> bool {
        active_search_section_override().is_some()
    }
}

/// Returns configured `web.search_backend` or shared `web.backend` when set.
pub fn active_search_section_override() -> Option<String> {
    use super::web_config::config_search_backend_choice;
    config_search_backend_choice()
}

/// Read search override from a specific config file (for tests and wizards).
pub fn search_section_override_from_path(config_path: &Path) -> Option<String> {
    use super::web_config::load_web_tools_config_from_path;
    let cfg = load_web_tools_config_from_path(config_path)?;
    if !cfg.search_backend.trim().is_empty() {
        return Some(cfg.search_backend);
    }
    if !cfg.backend.trim().is_empty() {
        return Some(cfg.backend);
    }
    None
}

/// Convert legacy `web.search_backend` / `web.backend` into `web_search` chain.
pub fn migrate_legacy_search_override(config_path: &Path) -> std::io::Result<bool> {
    let Some(name) = search_section_override_from_path(config_path) else {
        return Ok(false);
    };
    persist_search_backend_as_chain(config_path, &name)?;
    Ok(true)
}

/// Migrate legacy search overrides at startup (CLI/gateway). Idempotent.
pub fn ensure_web_search_config_coherence() {
    let path = resolve_edgecrab_home().join("config.yaml");
    ensure_web_search_config_coherence_at(&path);
}

/// Migrate legacy search overrides for a specific config path.
pub fn ensure_web_search_config_coherence_at(config_path: &Path) {
    match migrate_legacy_search_override(config_path) {
        Ok(true) => tracing::info!(
            path = %config_path.display(),
            "migrated legacy web.search_backend/web.backend to web_search chain"
        ),
        Ok(false) => {}
        Err(e) => tracing::warn!(
            path = %config_path.display(),
            error = %e,
            "web config coherence migration failed"
        ),
    }
}

/// User-facing warning when legacy `web:` search overrides are active.
pub fn search_override_warning() -> Option<String> {
    active_search_section_override().map(|name| {
        format!(
            "⚠ web.search_backend/web.backend={name} overrides your chain — reset with /web → a (auto) or re-save chain"
        )
    })
}

/// Default priority chain when user picks a single search backend in setup wizards.
pub fn default_chain_for_backend(backend: &str) -> Vec<String> {
    let id = backend.trim().to_ascii_lowercase();
    if id.is_empty() || id == "auto" {
        return vec!["ddgs".into()];
    }
    if id == "ddgs" {
        vec!["ddgs".into()]
    } else {
        vec![id, "ddgs".into()]
    }
}

/// Persist search routing as `web_search` chain (not `web.search_backend`).
pub fn persist_search_backend_as_chain(
    config_path: &Path,
    backend: &str,
) -> std::io::Result<()> {
    persist_search_chain_order(config_path, &default_chain_for_backend(backend))
}

// ─── Picker formatting ───────────────────────────────────────────────────

/// Format a picker row (name + badge + capabilities + optional ✓).
pub fn format_picker_label(row: &Value) -> String {
    let name = row["name"].as_str().unwrap_or("Unknown");
    let badge = row["badge"].as_str().filter(|b| !b.is_empty());
    let caps = capability_label(
        row["supports_search"].as_bool().unwrap_or(false),
        row["supports_extract"].as_bool().unwrap_or(false),
        row["supports_crawl"].as_bool().unwrap_or(false),
    );
    let configured = row["configured"].as_bool().unwrap_or(false);
    let mut label = match badge {
        Some(b) => format!("{name}  [{b}]  {caps}"),
        None => format!("{name}  {caps}"),
    };
    if configured {
        label.push_str("  ✓");
    }
    label
}

/// Plain-text detail lines for a provider (CLI wizard + TUI details panel).
pub fn provider_detail_lines(row: Option<&Value>, id: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(row) = row else {
        return lines;
    };

    lines.push(format_picker_label(row));
    if let Some(tag) = row["tag"].as_str().filter(|t| !t.is_empty()) {
        lines.push(tag.to_string());
    }

    if let Some(envs) = row["env_vars"].as_array().filter(|a| !a.is_empty()) {
        let mut any_missing = false;
        for ev in envs {
            let key = ev["key"].as_str().unwrap_or("");
            if key.is_empty() {
                continue;
            }
            let set = std::env::var(key)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            let mark = if set { "✓" } else { "·" };
            lines.push(format!("  {mark} {key}"));
            any_missing |= !set;
        }
        if any_missing {
            lines.push(String::new());
            lines.push("Add missing keys to ~/.edgecrab/.env".into());
        }
    } else if row["configured"].as_bool() == Some(true) {
        lines.push("✓ Credentials found".into());
    } else if id == "ddgs" {
        lines.push(
            "No API key. Often last resort — may hit bot checks on some networks.".into(),
        );
    }

    lines
}

/// Print provider detail block for the CLI wizard.
pub fn print_provider_detail_cli(row: &Value) {
    let id = row["id"].as_str().unwrap_or("");
    let name = row["name"].as_str().unwrap_or("Unknown");
    let caps = capability_label(
        row["supports_search"].as_bool().unwrap_or(false),
        row["supports_extract"].as_bool().unwrap_or(false),
        row["supports_crawl"].as_bool().unwrap_or(false),
    );
    println!("\n  ── {name} [{caps}] ──");
    let label = format_picker_label(row);
    for line in provider_detail_lines(Some(row), id) {
        if line.is_empty() || line == label {
            continue;
        }
        if line.starts_with("  ") {
            println!("{line}");
        } else {
            println!("  {line}");
        }
    }
    println!();
}

// ─── Chain persistence ───────────────────────────────────────────────────

pub fn chain_eligible_search_ids(rows: &[Value]) -> Vec<String> {
    rows.iter()
        .filter(|r| r["supports_search"].as_bool() == Some(true))
        .filter(|r| {
            let id = r["id"].as_str().unwrap_or("");
            id == "ddgs" || r["configured"].as_bool() == Some(true)
        })
        .filter_map(|r| r["id"].as_str().map(str::to_string))
        .collect()
}

pub fn order_from_primary_and_fallbacks(primary: &str, fallbacks: &[String]) -> Vec<String> {
    let mut chain = vec![primary.trim().to_ascii_lowercase()];
    for fb in fallbacks {
        let fb = fb.trim().to_ascii_lowercase();
        if !fb.is_empty() && !chain.contains(&fb) {
            chain.push(fb);
        }
    }
    chain
}

pub fn primary_and_fallbacks_from_order(order: &[String]) -> (String, Vec<String>) {
    let primary = order
        .first()
        .cloned()
        .unwrap_or_else(|| "ddgs".into());
    let fallbacks = order.iter().skip(1).cloned().collect();
    (primary, fallbacks)
}

pub fn persist_search_chain_order(config_path: &Path, order: &[String]) -> std::io::Result<()> {
    let (primary, fallbacks) = primary_and_fallbacks_from_order(order);
    let disk = load_web_search_config_from_disk();
    let update = WebSearchChainUpdate {
        primary: Some(primary),
        fallbacks: Some(fallbacks),
        timeout_secs: Some(disk.timeout_secs.max(8)),
    };
    clear_web_search_section_overrides(config_path)?;
    persist_web_search_chain_in_config(config_path, &update)
}

pub fn persist_search_chain_with_timeout(
    config_path: &Path,
    order: &[String],
    timeout_secs: u64,
) -> std::io::Result<()> {
    let (primary, fallbacks) = primary_and_fallbacks_from_order(order);
    clear_web_search_section_overrides(config_path)?;
    persist_web_search_chain_in_config(
        config_path,
        &WebSearchChainUpdate {
            primary: Some(primary),
            fallbacks: Some(fallbacks),
            timeout_secs: Some(timeout_secs.max(1)),
        },
    )
}

pub fn chain_summary_after_save(config_path: &Path) -> String {
    let disk = load_web_search_config_from_path(config_path)
        .unwrap_or_else(load_web_search_config_from_disk);
    format_search_chain_summary(&disk)
}

/// Clear `web:` overrides and `web_search` chain — restore full auto mode.
pub fn reset_web_to_auto(config_path: &Path) -> std::io::Result<()> {
    clear_web_section_overrides(config_path)?;
    clear_web_search_chain_in_config(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::web_config::{
        load_web_tools_config_from_path, persist_web_section_in_config, WebSectionUpdate,
    };
    use tempfile::TempDir;

    fn sample_row(id: &str, configured: bool) -> Value {
        serde_json::json!({
            "id": id,
            "name": id,
            "badge": "",
            "configured": configured,
            "supports_search": true,
            "supports_extract": false,
            "supports_crawl": false,
        })
    }

    #[test]
    fn format_picker_label_includes_capabilities() {
        let row = serde_json::json!({
            "name": "Firecrawl",
            "badge": "paid",
            "configured": false,
            "supports_search": true,
            "supports_extract": true,
            "supports_crawl": true,
        });
        let label = format_picker_label(&row);
        assert!(label.contains("Firecrawl"));
        assert!(label.contains("S+E+C"));
    }

    #[test]
    fn chain_eligible_includes_ddgs_without_config() {
        let rows = vec![
            sample_row("brave", false),
            sample_row("ddgs", false),
            sample_row("searxng", true),
        ];
        let ids = chain_eligible_search_ids(&rows);
        assert_eq!(ids, vec!["ddgs".to_string(), "searxng".to_string()]);
    }

    #[test]
    fn order_roundtrip_primary_fallbacks() {
        let order = vec!["searxng".into(), "brave".into(), "ddgs".into()];
        let (p, f) = primary_and_fallbacks_from_order(&order);
        assert_eq!(p, "searxng");
        assert_eq!(f, vec!["brave", "ddgs"]);
        assert_eq!(
            order_from_primary_and_fallbacks("searxng", &f),
            order
        );
    }

    #[test]
    fn editor_move_and_add() {
        let mut editor = WebChainEditor {
            catalog: WebPickerCatalog {
                rows: vec![
                    sample_row("searxng", true),
                    sample_row("brave", true),
                    sample_row("ddgs", false),
                ],
                chain_eligible_ids: vec!["searxng".into(), "brave".into(), "ddgs".into()],
            },
            order: vec!["searxng".into(), "ddgs".into()],
            is_auto: false,
        };
        assert_eq!(editor.move_item(1, -1).expect("move"), 0);
        assert_eq!(editor.order, vec!["ddgs", "searxng"]);
        editor.add_backend("brave").expect("add");
        assert!(editor.order.contains(&"brave".into()));
    }

    #[test]
    fn persist_chain_writes_yaml() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("config.yaml");
        persist_search_chain_with_timeout(&path, &["brave".into(), "ddgs".into()], 12)
            .expect("persist");
        let disk = load_web_search_config_from_path(&path).expect("load");
        assert_eq!(disk.primary, "brave");
        assert_eq!(disk.fallbacks, vec!["ddgs".to_string()]);
        assert_eq!(disk.timeout_secs, 12);
    }

    #[test]
    fn default_chain_for_paid_backend_appends_ddgs() {
        assert_eq!(
            default_chain_for_backend("brave"),
            vec!["brave".to_string(), "ddgs".to_string()]
        );
        assert_eq!(default_chain_for_backend("ddgs"), vec!["ddgs".to_string()]);
    }

    #[test]
    fn persist_chain_preserves_extract_backend() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("config.yaml");
        persist_web_section_in_config(
            &path,
            &WebSectionUpdate {
                backend: Some(String::new()),
                search_backend: Some(String::new()),
                extract_backend: Some("exa".into()),
            },
        )
        .expect("extract");
        persist_search_chain_order(&path, &["brave".into(), "ddgs".into()]).expect("chain");
        let web = load_web_tools_config_from_path(&path).expect("web");
        assert_eq!(web.extract_backend, "exa");
        assert!(web.search_backend.is_empty());
        let disk = load_web_search_config_from_path(&path).expect("search");
        assert_eq!(disk.primary, "brave");
    }

    #[test]
    fn migrate_legacy_search_override_converts_to_chain() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("config.yaml");
        persist_web_section_in_config(
            &path,
            &WebSectionUpdate {
                backend: Some(String::new()),
                search_backend: Some("tavily".into()),
                extract_backend: Some("exa".into()),
            },
        )
        .expect("seed");
        assert_eq!(
            search_section_override_from_path(&path).as_deref(),
            Some("tavily")
        );
        assert!(migrate_legacy_search_override(&path).expect("migrate"));
        assert!(search_section_override_from_path(&path).is_none());
        let web = load_web_tools_config_from_path(&path).expect("web");
        assert_eq!(web.extract_backend, "exa");
        let disk = load_web_search_config_from_path(&path).expect("search");
        assert_eq!(disk.primary, "tavily");
        assert_eq!(disk.fallbacks, vec!["ddgs".to_string()]);
    }

    #[test]
    fn persist_search_backend_as_chain_clears_search_override() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("config.yaml");
        persist_web_section_in_config(
            &path,
            &WebSectionUpdate {
                backend: Some(String::new()),
                search_backend: Some("tavily".into()),
                extract_backend: None,
            },
        )
        .expect("override");
        persist_search_backend_as_chain(&path, "brave").expect("chain");
        let web = load_web_tools_config_from_path(&path).expect("web");
        assert!(web.search_backend.is_empty());
        let disk = load_web_search_config_from_path(&path).expect("search");
        assert_eq!(disk.primary, "brave");
        assert_eq!(disk.fallbacks, vec!["ddgs".to_string()]);
    }
}
