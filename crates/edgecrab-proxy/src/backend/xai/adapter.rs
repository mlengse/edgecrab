//! xAI Grok OAuth upstream (Hermes `XAIGrokAdapter`).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::backend::adapter::{UpstreamAdapter, UpstreamCredential};
use crate::backend::auth_file::{default_auth_path, load_auth_doc, read_credential_pool_entries};
use crate::backend::xai::refresh::{resolve_xai_credentials_async, DEFAULT_XAI_API};
use crate::error::ProxyError;

/// OAuth-capable xAI forward upstream with pool rotation on 429.
pub struct XaiGrokAdapter {
    name: String,
    auth_provider: String,
    auth_path: PathBuf,
    fallback_base_url: String,
    auth_hint: String,
    lock: Mutex<()>,
    pool_index: AtomicUsize,
}

impl XaiGrokAdapter {
    pub fn new(
        upstream_key: &str,
        auth_provider: Option<&str>,
        auth_path: Option<PathBuf>,
        fallback_base_url: String,
        auth_hint: Option<String>,
    ) -> Self {
        let provider = auth_provider
            .map(str::to_string)
            .unwrap_or_else(|| "xai-oauth".to_string());
        let hint = auth_hint.unwrap_or_else(|| {
            "run `hermes auth add xai-oauth --type oauth`".to_string()
        });
        Self {
            name: upstream_key.to_string(),
            auth_provider: provider,
            auth_path: auth_path.unwrap_or_else(default_auth_path),
            fallback_base_url: fallback_base_url.trim_end_matches('/').to_string(),
            auth_hint: hint,
            lock: Mutex::new(()),
            pool_index: AtomicUsize::new(0),
        }
    }

    fn pool_len(&self) -> usize {
        load_auth_doc(&self.auth_path)
            .ok()
            .map(|doc| read_credential_pool_entries(&doc, &self.auth_provider).len())
            .unwrap_or(0)
    }

    async fn credential_inner(&self, force_refresh: bool) -> Result<UpstreamCredential, ProxyError> {
        let _guard = self.lock.lock().await;
        let idx = self.pool_index.load(Ordering::SeqCst);
        let fallback = if self.fallback_base_url.is_empty() {
            DEFAULT_XAI_API
        } else {
            &self.fallback_base_url
        };
        let (bearer, base_url) = resolve_xai_credentials_async(
            &self.auth_path,
            &self.auth_provider,
            fallback,
            idx,
            force_refresh,
        )
        .await?;
        Ok(UpstreamCredential {
            bearer,
            base_url,
            token_type: "Bearer".into(),
        })
    }
}

#[async_trait]
impl UpstreamAdapter for XaiGrokAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        "xAI Grok OAuth"
    }

    fn allowed_paths(&self) -> HashSet<String> {
        HashSet::from([
            "/responses".into(),
            "/chat/completions".into(),
            "/completions".into(),
            "/embeddings".into(),
            "/models".into(),
        ])
    }

    fn is_authenticated(&self) -> bool {
        let Ok(doc) = load_auth_doc(&self.auth_path) else {
            return false;
        };
        let pool = read_credential_pool_entries(&doc, &self.auth_provider);
        if pool.iter().any(|e| {
            e.get("access_token")
                .or_else(|| e.get("runtime_api_key"))
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty())
        }) {
            return true;
        }
        crate::backend::auth_store::provider_state_from_doc(&doc, &self.auth_provider)
            .map(|s| {
                s.get("tokens")
                    .and_then(|t| t.get("access_token"))
                    .and_then(|v| v.as_str())
                    .is_some_and(|tok| !tok.is_empty())
                    || s.get("refresh_token")
                        .and_then(|v| v.as_str())
                        .is_some_and(|rt| !rt.is_empty())
                    || s.get("tokens")
                        .and_then(|t| t.get("refresh_token"))
                        .and_then(|v| v.as_str())
                        .is_some_and(|rt| !rt.is_empty())
            })
            .unwrap_or(false)
    }

    async fn get_credential(&self) -> Result<UpstreamCredential, ProxyError> {
        self.credential_inner(false).await
    }

    async fn get_retry_credential(
        &self,
        failed: &UpstreamCredential,
        status_code: u16,
    ) -> Option<UpstreamCredential> {
        if status_code == 429 {
            let len = self.pool_len();
            if len > 1 {
                let next = (self.pool_index.load(Ordering::SeqCst) + 1) % len;
                self.pool_index.store(next, Ordering::SeqCst);
                tracing::info!("proxy: xAI upstream 429; rotating to credential pool index {next}");
                if let Ok(cred) = self.credential_inner(false).await
                    && cred.bearer != failed.bearer
                {
                    return Some(cred);
                }
            }
            return None;
        }
        if status_code != 401 {
            return None;
        }
        tracing::info!("proxy: xAI upstream 401; refreshing OAuth token");
        self.credential_inner(true).await.ok().filter(|c| c.bearer != failed.bearer)
    }

    fn auth_hint(&self) -> String {
        self.auth_hint.clone()
    }
}
