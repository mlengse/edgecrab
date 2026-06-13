//! Upstream adapter contract (Hermes `hermes_cli/proxy/adapters/base.py`).

use std::collections::HashSet;

use async_trait::async_trait;

use crate::error::ProxyError;

/// Resolved bearer + base URL for an upstream forward request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamCredential {
    /// Token value only (no `Bearer` prefix).
    pub bearer: String,
    pub base_url: String,
    pub token_type: String,
}

impl UpstreamCredential {
    pub fn authorization_header(&self) -> String {
        format!("{} {}", self.token_type, self.bearer)
    }
}

/// OAuth-capable upstream; Mode A forwarder depends on this trait (DIP).
#[async_trait]
pub trait UpstreamAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn allowed_paths(&self) -> HashSet<String>;
    fn is_authenticated(&self) -> bool;
    async fn get_credential(&self) -> Result<UpstreamCredential, ProxyError>;
    async fn get_retry_credential(
        &self,
        _failed: &UpstreamCredential,
        _status_code: u16,
    ) -> Option<UpstreamCredential> {
        None
    }

    /// CLI hint when `is_authenticated()` is false (Hermes adapter pattern).
    fn auth_hint(&self) -> String {
        format!(
            "configure proxy.forward_upstreams.{}.bearer_env or bearer",
            self.name()
        )
    }
}

/// Static bearer for tests and env-backed forwards until 024 OAuth adapters land.
pub struct StaticBearerAdapter {
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub bearer: String,
    pub auth_hint: String,
    paths: HashSet<String>,
}

impl StaticBearerAdapter {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        bearer: impl Into<String>,
    ) -> Self {
        Self::with_auth_hint(name, base_url, bearer, None)
    }

    pub fn with_auth_hint(
        name: impl Into<String>,
        base_url: impl Into<String>,
        bearer: impl Into<String>,
        auth_hint: Option<String>,
    ) -> Self {
        let name = name.into();
        let hint = auth_hint.unwrap_or_else(|| {
            format!("set proxy.forward_upstreams.{name}.bearer_env or bearer in config.yaml")
        });
        Self {
            display_name: name.clone(),
            name,
            base_url: base_url.into(),
            bearer: bearer.into(),
            auth_hint: hint,
            paths: HashSet::from([
                "/chat/completions".into(),
                "/models".into(),
                "/embeddings".into(),
            ]),
        }
    }
}

#[async_trait]
impl UpstreamAdapter for StaticBearerAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn allowed_paths(&self) -> HashSet<String> {
        self.paths.clone()
    }

    fn is_authenticated(&self) -> bool {
        !self.bearer.is_empty()
    }

    async fn get_credential(&self) -> Result<UpstreamCredential, ProxyError> {
        if !self.is_authenticated() {
            return Err(ProxyError::UpstreamAuth(format!(
                "upstream '{}' is not authenticated",
                self.name
            )));
        }
        Ok(UpstreamCredential {
            bearer: self.bearer.clone(),
            base_url: self.base_url.clone(),
            token_type: "Bearer".into(),
        })
    }

    fn auth_hint(&self) -> String {
        self.auth_hint.clone()
    }
}

/// One-line status for `edgecrab proxy status` (Hermes `UpstreamAdapter.describe`).
pub fn describe_adapter(adapter: &dyn UpstreamAdapter) -> String {
    if !adapter.is_authenticated() {
        return format!("{} — not authenticated", adapter.display_name());
    }
    match block_on_credential(adapter) {
        Ok(cred) => format!("{} — {} (ready)", adapter.display_name(), cred.base_url),
        Err(err) => format!("{} — not ready ({err})", adapter.display_name()),
    }
}

/// Run async credential resolution from sync CLI paths (`proxy status`).
pub fn block_on_credential(
    adapter: &dyn UpstreamAdapter,
) -> Result<UpstreamCredential, ProxyError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ProxyError::Upstream(format!("adapter runtime: {e}")))?;
    rt.block_on(adapter.get_credential())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_ready_upstream() {
        let a = StaticBearerAdapter::new("nous", "https://api.example/v1", "tok");
        let line = describe_adapter(&a);
        assert!(line.contains("ready"));
        assert!(line.contains("api.example"));
    }
}
