//! Hermes-aligned `web:` config section — per-capability backend overrides.
//!
//! ```yaml
//! web:
//!   search_backend: ""   # overrides web_search.primary when set
//!   extract_backend: ""  # default backend for web_extract / web_crawl
//!   backend: ""          # shared fallback for both capabilities
//! ```

use std::path::Path;

use crate::config_ref::WebToolsConfigRef;
use crate::tools::web::search::backend_settings::{
    backend_is_configured, lookup_backend_config, normalize_backend_name,
};
use crate::tools::web::search::config::load_web_search_config_from_disk;
use crate::tools::web::search::provider_capabilities;

/// Load `web:` section from config.yaml (empty strings when section missing).
pub fn load_web_tools_config_from_disk() -> WebToolsConfigRef {
    let path = crate::config_ref::resolve_edgecrab_home().join("config.yaml");
    load_web_tools_config_from_path(&path).unwrap_or_default()
}

pub fn load_web_tools_config_from_path(path: &Path) -> Option<WebToolsConfigRef> {
    let content = std::fs::read_to_string(path).ok()?;
    let raw: serde_yml::Value = serde_yml::from_str(&content).ok()?;
    let section = raw.get("web")?;
    serde_yml::from_value(section.clone()).ok()
}

fn normalized_choice(value: &str) -> Option<String> {
    let v = value.trim().to_ascii_lowercase();
    if v.is_empty() || v == "auto" {
        None
    } else {
        Some(normalize_backend_name(&v))
    }
}

/// Configured extract backend: `web.extract_backend` → `web.backend`.
pub fn config_extract_backend_choice() -> Option<String> {
    let cfg = load_web_tools_config_from_disk();
    normalized_choice(&cfg.extract_backend).or_else(|| normalized_choice(&cfg.backend))
}

/// Configured search backend: `web.search_backend` → `web.backend`.
pub fn config_search_backend_choice() -> Option<String> {
    let cfg = load_web_tools_config_from_disk();
    normalized_choice(&cfg.search_backend).or_else(|| normalized_choice(&cfg.backend))
}

/// True when the named backend has credentials / env configured for extract routing.
pub fn extract_backend_is_available(name: &str) -> bool {
    let name = normalize_backend_name(name);
    match name.as_str() {
        "native" | "browser" => true,
        "firecrawl" | "parallel" | "tavily" | "exa" => {
            let disk = load_web_search_config_from_disk();
            backend_is_configured(&name, &lookup_backend_config(&disk.backends, &name))
        }
        _ => false,
    }
}

/// True when the named backend has credentials / env configured for search routing.
pub fn search_backend_is_available(name: &str) -> bool {
    let name = normalize_backend_name(name);
    if name == "ddgs" {
        return true;
    }
    let disk = load_web_search_config_from_disk();
    backend_is_configured(&name, &lookup_backend_config(&disk.backends, &name))
}

/// Resolve search backend from config when env/tool arg did not override.
pub fn resolve_config_search_backend() -> Option<String> {
    config_search_backend_choice()
        .filter(|name| name != "ddgs")
        .filter(|name| provider_capabilities::supports_search(name))
        .filter(|name| search_backend_is_available(name))
}

/// Resolve extract backend from config when env/tool arg did not override.
pub fn resolve_config_extract_backend() -> Option<String> {
    config_extract_backend_choice()
        .filter(|name| provider_capabilities::supports_extract(name))
        .filter(|name| extract_backend_is_available(name))
}

/// Persist Hermes-style shared `web.backend` choice (clears per-capability overrides).
pub fn persist_web_backend_in_config(
    config_path: &Path,
    backend_id: &str,
) -> Result<(), std::io::Error> {
    persist_web_section_in_config(
        config_path,
        &WebSectionUpdate {
            backend: Some(backend_id.to_string()),
            search_backend: Some(String::new()),
            extract_backend: Some(String::new()),
        },
    )
}

/// Partial update for the `web:` config section (unset fields are left unchanged).
#[derive(Debug, Clone, Default)]
pub struct WebSectionUpdate {
    pub backend: Option<String>,
    pub search_backend: Option<String>,
    pub extract_backend: Option<String>,
}

fn insert_web_field(map: &mut serde_yml::Mapping, key: &str, value: &str) {
    map.insert(
        serde_yml::Value::String(key.into()),
        serde_yml::Value::String(value.to_string()),
    );
}

/// Merge a partial `web:` update into config.yaml (creates section if missing).
pub fn persist_web_section_in_config(
    config_path: &Path,
    update: &WebSectionUpdate,
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
    let mapping = raw
        .as_mapping_mut()
        .ok_or_else(|| std::io::Error::other("config root must be a mapping"))?;

    let mut web_map = mapping
        .get(serde_yml::Value::String("web".into()))
        .and_then(|v| v.as_mapping())
        .cloned()
        .unwrap_or_default();

    if let Some(ref backend) = update.backend {
        insert_web_field(&mut web_map, "backend", &normalize_backend_name(backend));
    }
    if let Some(ref search) = update.search_backend {
        insert_web_field(
            &mut web_map,
            "search_backend",
            &search.trim().to_ascii_lowercase(),
        );
    }
    if let Some(ref extract) = update.extract_backend {
        insert_web_field(
            &mut web_map,
            "extract_backend",
            &extract.trim().to_ascii_lowercase(),
        );
    }

    mapping.insert(
        serde_yml::Value::String("web".into()),
        serde_yml::Value::Mapping(web_map),
    );

    let serialized = serde_yml::to_string(&raw).map_err(std::io::Error::other)?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(config_path, serialized)
}

/// Clear search-related `web:` overrides only — preserves `extract_backend`.
pub fn clear_web_search_section_overrides(config_path: &Path) -> Result<(), std::io::Error> {
    persist_web_section_in_config(
        config_path,
        &WebSectionUpdate {
            backend: Some(String::new()),
            search_backend: Some(String::new()),
            extract_backend: None,
        },
    )
}

/// Clear all `web:` overrides (auto routing for search + extract).
pub fn clear_web_section_overrides(config_path: &Path) -> Result<(), std::io::Error> {
    persist_web_section_in_config(
        config_path,
        &WebSectionUpdate {
            backend: Some(String::new()),
            search_backend: Some(String::new()),
            extract_backend: Some(String::new()),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_isolation::{EdgecrabHomeGuard, web_config_test_lock};
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn clear_web_search_section_preserves_extract_backend() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.yaml");
        persist_web_section_in_config(
            &path,
            &WebSectionUpdate {
                backend: Some("tavily".into()),
                search_backend: Some("brave".into()),
                extract_backend: Some("exa".into()),
            },
        )
        .expect("seed");
        clear_web_search_section_overrides(&path).expect("clear search");
        let cfg = load_web_tools_config_from_path(&path).expect("parse");
        assert!(cfg.backend.is_empty());
        assert!(cfg.search_backend.is_empty());
        assert_eq!(cfg.extract_backend, "exa");
    }

    #[test]
    fn load_web_section_extract_and_shared_backend() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.yaml");
        let mut file = std::fs::File::create(&path).expect("create");
        write!(
            file,
            r#"
web:
  extract_backend: exa
  backend: tavily
  search_backend: brave
"#
        )
        .expect("write");
        let cfg = load_web_tools_config_from_path(&path).expect("parse");
        assert_eq!(cfg.extract_backend, "exa");
        assert_eq!(cfg.backend, "tavily");
        assert_eq!(cfg.search_backend, "brave");
    }

    #[test]
    fn extract_backend_prefers_specific_over_shared() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(Some(
            r#"
web:
  extract_backend: parallel
  backend: tavily
"#,
        ));
        let prev_exa = std::env::var("EXA_API_KEY").ok();
        let prev_par = std::env::var("PARALLEL_API_KEY").ok();
        unsafe { std::env::remove_var("EXA_API_KEY") };
        unsafe { std::env::set_var("PARALLEL_API_KEY", "test-key") };

        assert_eq!(config_extract_backend_choice().as_deref(), Some("parallel"));

        unsafe { std::env::remove_var("PARALLEL_API_KEY") };
        if let Some(v) = prev_par {
            unsafe { std::env::set_var("PARALLEL_API_KEY", v) };
        }
        if let Some(v) = prev_exa {
            unsafe { std::env::set_var("EXA_API_KEY", v) };
        }
    }

    #[test]
    fn unavailable_config_extract_backend_not_forced() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(Some(
            r#"
web:
  extract_backend: exa
"#,
        ));
        let prev = std::env::var("EXA_API_KEY").ok();
        unsafe { std::env::remove_var("EXA_API_KEY") };
        assert!(resolve_config_extract_backend().is_none());
        if let Some(v) = prev {
            unsafe { std::env::set_var("EXA_API_KEY", v) };
        }
    }

    #[test]
    fn configured_extract_backend_from_yaml_and_credentials() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(Some(
            r#"
web_search:
  backends:
    exa:
      api_key: cfg-exa-key
web:
  extract_backend: exa
"#,
        ));
        let prev = std::env::var("EXA_API_KEY").ok();
        unsafe { std::env::remove_var("EXA_API_KEY") };
        assert_eq!(resolve_config_extract_backend().as_deref(), Some("exa"));
        if let Some(v) = prev {
            unsafe { std::env::set_var("EXA_API_KEY", v) };
        }
    }

    #[test]
    fn persist_web_section_split_backends() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.yaml");
        persist_web_section_in_config(
            &path,
            &WebSectionUpdate {
                backend: Some(String::new()),
                search_backend: Some("brave".into()),
                extract_backend: Some("exa".into()),
            },
        )
        .expect("persist");
        let cfg = load_web_tools_config_from_path(&path).expect("parse");
        assert_eq!(cfg.search_backend, "brave");
        assert_eq!(cfg.extract_backend, "exa");
        assert!(cfg.backend.is_empty());
    }

    #[test]
    fn persist_web_backend_writes_yaml() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("config.yaml");
        persist_web_backend_in_config(&path, "tavily").expect("persist");
        let cfg = load_web_tools_config_from_path(&path).expect("parse");
        assert_eq!(cfg.backend, "tavily");
    }

    #[test]
    fn search_only_extract_config_is_not_selected() {
        let _lock = web_config_test_lock();
        let _home = EdgecrabHomeGuard::isolated(Some(
            r#"
web_search:
  backends:
    brave:
      api_key: brave-key
web:
  extract_backend: brave
"#,
        ));
        let prev = std::env::var("BRAVE_API_KEY").ok();
        unsafe { std::env::remove_var("BRAVE_API_KEY") };
        unsafe { std::env::remove_var("BRAVE_SEARCH_API_KEY") };
        assert!(resolve_config_extract_backend().is_none());
        if let Some(v) = prev {
            unsafe { std::env::set_var("BRAVE_API_KEY", v) };
        }
    }
}
