//! E2E: web_search config loaded from config.yaml on disk.

mod common;

use common::registry_guard;
use edgecrab_tools::tools::web::search::config::load_web_search_config_from_path;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn e2e_config_yaml_primary_fallbacks_and_timeout() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("config.yaml");
    let mut file = std::fs::File::create(&path).expect("create config");
    write!(
        file,
        r#"
web_search:
  primary: searxng
  fallbacks: [brave, ddgs]
  timeout_secs: 12
  backends:
    brave:
      rps: 2.5
"#
    )
    .expect("write config");

    let cfg = load_web_search_config_from_path(&path).expect("parse config");
    assert_eq!(cfg.primary, "searxng");
    assert_eq!(cfg.fallbacks, vec!["brave", "ddgs"]);
    assert_eq!(cfg.timeout_secs, 12);
    assert_eq!(cfg.backends.get("brave").and_then(|b| b.rps), Some(2.5));
}

#[test]
fn e2e_config_yaml_missing_section_returns_none() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("config.yaml");
    std::fs::write(&path, "model: foo\n").expect("write");

    assert!(load_web_search_config_from_path(&path).is_none());
}
