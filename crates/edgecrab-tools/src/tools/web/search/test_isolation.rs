//! Serializes env + `EDGECRAB_HOME` mutations in web search / extract tests.

use std::sync::{Mutex, MutexGuard};
use tempfile::TempDir;

static WEB_CONFIG_TEST_LOCK: Mutex<()> = Mutex::new(());

pub fn web_config_test_lock() -> MutexGuard<'static, ()> {
    WEB_CONFIG_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Temporarily point `EDGECRAB_HOME` at an isolated `config.yaml`.
pub struct EdgecrabHomeGuard {
    prev_home: Option<String>,
    _dir: TempDir,
}

impl EdgecrabHomeGuard {
    pub fn isolated(yaml: Option<&str>) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let content = yaml.unwrap_or(
            r#"
web_search:
  primary: ""
"#,
        );
        std::fs::write(dir.path().join("config.yaml"), content).expect("write config");
        let prev_home = std::env::var("EDGECRAB_HOME").ok();
        unsafe { std::env::set_var("EDGECRAB_HOME", dir.path()) };
        Self {
            prev_home,
            _dir: dir,
        }
    }
}

impl Drop for EdgecrabHomeGuard {
    fn drop(&mut self) {
        unsafe { std::env::remove_var("EDGECRAB_HOME") };
        if let Some(v) = &self.prev_home {
            unsafe { std::env::set_var("EDGECRAB_HOME", v) };
        }
    }
}
