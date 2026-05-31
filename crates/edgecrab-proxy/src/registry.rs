//! Forward upstream registry (Hermes `ADAPTERS` / `get_adapter` pattern).

use std::collections::HashMap;
use std::sync::Arc;

use edgecrab_core::ProxyConfig;

use crate::backend::adapter::{UpstreamAdapter, describe_adapter};
use crate::error::ProxyError;
use crate::resolve::build_forward_adapters;

/// Sorted keys from `proxy.forward_upstreams`.
pub fn list_forward_upstream_keys(cfg: &ProxyConfig) -> Vec<String> {
    let mut keys: Vec<_> = cfg.forward_upstreams.keys().cloned().collect();
    keys.sort();
    keys
}

/// Resolve a configured forward upstream or return a clear error.
pub fn get_forward_adapter<'a>(
    adapters: &'a HashMap<String, Arc<dyn UpstreamAdapter>>,
    key: &str,
) -> Result<&'a Arc<dyn UpstreamAdapter>, ProxyError> {
    adapters.get(key).ok_or_else(|| {
        let available = {
            let mut k: Vec<_> = adapters.keys().cloned().collect();
            k.sort();
            k.join(", ")
        };
        ProxyError::ModelNotFound(format!(
            "unknown forward upstream '{key}'. Configured: [{available}]"
        ))
    })
}

/// Hermes `proxy start` preflight: upstream must be authenticated before bind.
pub async fn ensure_forward_upstream_ready(
    adapters: &HashMap<String, Arc<dyn UpstreamAdapter>>,
    key: &str,
) -> Result<(), ProxyError> {
    let adapter = get_forward_adapter(adapters, key)?;
    if !adapter.is_authenticated() {
        return Err(ProxyError::UpstreamAuth(format!(
            "{} is not authenticated — {}",
            adapter.display_name(),
            adapter.auth_hint()
        )));
    }
    let _ = adapter.get_credential().await?;
    Ok(())
}

/// Hermes `ADAPTERS` registry — documented upstream keys when config is empty.
pub fn builtin_upstream_catalog_lines() -> Vec<String> {
    let mut lines: Vec<String> = crate::guide::ALL_RECIPES
        .iter()
        .map(|r| {
            format!(
                "  {:4} → {}  (`edgecrab proxy enable {}`)",
                r.key, r.display_name, r.key
            )
        })
        .collect();
    lines.push("  *    → static / hermes_auth (API key or read-only auth.json)".into());
    lines
}

/// Status lines for `edgecrab proxy status` / `edgecrab proxy upstreams`.
pub fn format_upstream_status_table(cfg: &ProxyConfig) -> Vec<String> {
    let adapters = build_forward_adapters(&cfg.forward_upstreams);
    let mut lines = Vec::new();
    for key in list_forward_upstream_keys(cfg) {
        if let Some(adapter) = adapters.get(&key) {
            lines.push(format!("  [{key:12}] {}", describe_adapter(adapter.as_ref())));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgecrab_core::ForwardUpstreamConfig;

    #[tokio::test]
    async fn ensure_ready_rejects_empty_bearer() {
        let mut cfg = ProxyConfig::default();
        cfg.forward_upstreams.insert(
            "empty".into(),
            ForwardUpstreamConfig {
                base_url: "http://x/v1".into(),
                ..Default::default()
            },
        );
        let adapters = build_forward_adapters(&cfg.forward_upstreams);
        let err = ensure_forward_upstream_ready(&adapters, "empty")
            .await
            .unwrap_err();
        assert!(matches!(err, ProxyError::UpstreamAuth(_)));
    }

    #[test]
    fn builtin_catalog_lists_hermes_providers() {
        let lines = builtin_upstream_catalog_lines();
        assert!(lines.iter().any(|l| l.contains("nous")));
        assert!(lines.iter().any(|l| l.contains("xai")));
    }

    #[test]
    fn lists_sorted_upstream_keys() {
        let mut cfg = ProxyConfig::default();
        cfg.forward_upstreams.insert(
            "zeta".into(),
            ForwardUpstreamConfig {
                base_url: "http://z/v1".into(),
                bearer: Some("t".into()),
                ..Default::default()
            },
        );
        cfg.forward_upstreams.insert(
            "alpha".into(),
            ForwardUpstreamConfig {
                base_url: "http://a/v1".into(),
                bearer: Some("t".into()),
                ..Default::default()
            },
        );
        assert_eq!(list_forward_upstream_keys(&cfg), vec!["alpha", "zeta"]);
    }
}
