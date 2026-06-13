//! Loaded proxy configuration + adapters (single load site — DRY).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use edgecrab_core::AppConfig;
use edgecrab_core::config::ensure_edgecrab_home;
use edgecrab_proxy::backend::adapter::UpstreamAdapter;
use edgecrab_proxy::{
    build_forward_adapters, ensure_forward_upstream_ready, ensure_proxy_token,
    list_forward_upstream_keys, load_proxy_token,
};

pub struct ProxySession {
    pub app: AppConfig,
    pub token_path: PathBuf,
    pub adapters: HashMap<String, Arc<dyn UpstreamAdapter>>,
}

impl ProxySession {
    pub fn load() -> Result<Self> {
        let app = AppConfig::load().unwrap_or_default();
        let token_path = app.proxy.resolved_token_path();
        let adapters = build_forward_adapters(&app.proxy.forward_upstreams);
        Ok(Self {
            app,
            token_path,
            adapters,
        })
    }

    pub fn proxy(&self) -> &edgecrab_core::ProxyConfig {
        &self.app.proxy
    }

    pub fn config_path() -> PathBuf {
        ensure_edgecrab_home()
            .map(|h| h.join("config.yaml"))
            .unwrap_or_else(|_| PathBuf::from("~/.edgecrab/config.yaml"))
    }

    pub fn save_mut(&mut self) -> Result<()> {
        self.app
            .save()
            .map_err(|e| anyhow::anyhow!("failed to save config: {e}"))
    }

    pub fn token_present(&self) -> bool {
        self.token_path.exists()
    }

    pub fn ensure_token(&self) -> Result<String> {
        if self.token_path.exists() {
            return load_proxy_token(&self.token_path).context("read proxy token");
        }
        bail!(
            "no proxy token at {} — run `edgecrab proxy token set` or `edgecrab proxy setup`",
            self.token_path.display()
        );
    }

    pub fn ensure_token_create(&self) -> Result<String> {
        ensure_proxy_token(&self.token_path).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn default_model_spec(&self) -> Option<String> {
        if self.app.model.default_model.contains('/') {
            Some(self.app.model.default_model.clone())
        } else {
            None
        }
    }

    pub fn upstream_keys(&self) -> Vec<String> {
        list_forward_upstream_keys(self.proxy())
    }

    pub async fn ensure_upstream_ready(&self, key: &str) -> Result<()> {
        ensure_forward_upstream_ready(&self.adapters, key)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}
