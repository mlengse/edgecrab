#![allow(clippy::await_holding_lock)]
#![allow(dead_code)]
//! E2E: `edgecrab setup web` config persistence (Hermes tools picker write path).

mod common;

use common::registry_guard;
use edgecrab_tools::tools::web::search::{
    load_web_tools_config_from_path, persist_web_backend_in_config,
};
use tempfile::TempDir;

#[test]
fn e2e_setup_web_persists_backend_in_config_yaml() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("config.yaml");
    persist_web_backend_in_config(&path, "firecrawl").expect("persist");
    let cfg = load_web_tools_config_from_path(&path).expect("parse web section");
    assert_eq!(cfg.backend, "firecrawl");
}

#[test]
fn e2e_setup_web_normalizes_brave_free_alias() {
    let _lock = registry_guard();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("config.yaml");
    persist_web_backend_in_config(&path, "brave-free").expect("persist");
    let cfg = load_web_tools_config_from_path(&path).expect("parse");
    assert_eq!(cfg.backend, "brave");
}
