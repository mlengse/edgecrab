//! Nous Portal upstream adapter (Hermes `NousPortalAdapter`).

use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use crate::backend::adapter::{UpstreamAdapter, UpstreamCredential};
use crate::backend::auth_file::{default_auth_path, read_provider_state};
use crate::backend::nous::quarantine::state_requires_relogin;
use crate::backend::nous::refresh::{DEFAULT_NOUS_INFERENCE, resolve_nous_credentials_async};
use crate::error::ProxyError;

/// OAuth-capable Nous Portal forward upstream with refresh-on-401.
pub struct NousPortalAdapter {
    name: String,
    auth_provider: String,
    auth_path: PathBuf,
    fallback_base_url: String,
    auth_hint: String,
    lock: Mutex<()>,
}

impl NousPortalAdapter {
    pub fn new(
        upstream_key: &str,
        auth_provider: Option<&str>,
        auth_path: Option<PathBuf>,
        fallback_base_url: String,
        auth_hint: Option<String>,
    ) -> Self {
        let provider = auth_provider
            .map(str::to_string)
            .unwrap_or_else(|| upstream_key.to_string());
        let hint = auth_hint.unwrap_or_else(|| format!("run `edgecrab auth add {provider}`"));
        Self {
            name: upstream_key.to_string(),
            auth_provider: provider,
            auth_path: auth_path.unwrap_or_else(default_auth_path),
            fallback_base_url: fallback_base_url.trim_end_matches('/').to_string(),
            auth_hint: hint,
            lock: Mutex::new(()),
        }
    }

    async fn credential_inner(
        &self,
        force_refresh: bool,
    ) -> Result<UpstreamCredential, ProxyError> {
        let _guard = self.lock.lock().await;
        let fallback = if self.fallback_base_url.is_empty() {
            DEFAULT_NOUS_INFERENCE
        } else {
            &self.fallback_base_url
        };
        let (bearer, base_url) = resolve_nous_credentials_async(
            &self.auth_path,
            &self.auth_provider,
            fallback,
            force_refresh,
        )
        .await?;
        Ok(UpstreamCredential {
            bearer,
            base_url,
            token_type: "Bearer".into(),
        })
    }

    fn state_snapshot(&self) -> Result<Value, ProxyError> {
        read_provider_state(&self.auth_path, &self.auth_provider)
    }
}

#[async_trait]
impl UpstreamAdapter for NousPortalAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        "Nous Portal"
    }

    fn allowed_paths(&self) -> HashSet<String> {
        HashSet::from([
            "/chat/completions".into(),
            "/completions".into(),
            "/embeddings".into(),
            "/models".into(),
        ])
    }

    fn is_authenticated(&self) -> bool {
        let Ok(state) = self.state_snapshot() else {
            return false;
        };
        if state_requires_relogin(&state) {
            return false;
        }
        if state
            .get("agent_key")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
        {
            return true;
        }
        state
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
            && state
                .get("access_token")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty())
    }

    async fn get_credential(&self) -> Result<UpstreamCredential, ProxyError> {
        self.credential_inner(false).await
    }

    async fn get_retry_credential(
        &self,
        _failed: &UpstreamCredential,
        status_code: u16,
    ) -> Option<UpstreamCredential> {
        if status_code != 401 {
            return None;
        }
        tracing::info!("proxy: Nous upstream rejected bearer; force-refreshing invoke JWT");
        self.credential_inner(true).await.ok()
    }

    fn auth_hint(&self) -> String {
        self.auth_hint.clone()
    }
}
