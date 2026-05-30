//! E2E: split web.search_backend / web.extract_backend persistence.

mod common;

use common::registry_guard;
use edgecrab_tools::{
    WebSectionUpdate, persist_web_section_in_config,
    tools::web::search::load_web_tools_config_from_path,
};
use tempfile::TempDir;

#[test]
fn e2e_setup_web_split_search_and_extract_backends() {
    let _lock = registry_guard();
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
}

#[test]
fn e2e_format_web_setup_report_includes_capability_column() {
    let _lock = registry_guard();
    let report = edgecrab_tools::collect_web_diagnostics();
    let text = edgecrab_tools::format_web_setup_report(&report);
    assert!(text.contains("S+E") || text.contains("[S]"));
}
