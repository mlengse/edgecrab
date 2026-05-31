//! Factory for forward upstream adapters (OCP — 024 OAuth kinds register here).

use std::collections::HashMap;
use std::sync::Arc;

use edgecrab_core::{ForwardAdapterKind, ForwardUpstreamConfig};

use super::adapter::StaticBearerAdapter;
use super::adapter::UpstreamAdapter;
use super::auth_store::HermesAuthFileAdapter;
use super::nous::NousPortalAdapter;
use super::xai::XaiGrokAdapter;

pub fn build_forward_adapter(
    key: &str,
    cfg: &ForwardUpstreamConfig,
) -> Arc<dyn UpstreamAdapter> {
    match cfg.adapter {
        ForwardAdapterKind::XaiOauth => Arc::new(XaiGrokAdapter::new(
            key,
            cfg.auth_provider.as_deref(),
            cfg.auth_path.clone(),
            cfg.base_url.clone(),
            cfg.auth_hint.clone(),
        )),
        ForwardAdapterKind::NousPortal => Arc::new(NousPortalAdapter::new(
            key,
            cfg.auth_provider.as_deref(),
            cfg.auth_path.clone(),
            cfg.base_url.clone(),
            cfg.auth_hint.clone(),
        )),
        ForwardAdapterKind::HermesAuth => Arc::new(HermesAuthFileAdapter::new(
            key,
            cfg.auth_provider.as_deref(),
            cfg.auth_path.clone(),
            cfg.base_url.clone(),
            cfg.auth_hint.clone(),
        )),
        ForwardAdapterKind::Static => {
            let bearer = cfg
                .bearer
                .clone()
                .or_else(|| {
                    cfg.bearer_env
                        .as_ref()
                        .and_then(|name| std::env::var(name).ok())
                })
                .unwrap_or_default();
            Arc::new(StaticBearerAdapter::with_auth_hint(
                key,
                &cfg.base_url,
                bearer,
                cfg.auth_hint.clone(),
            ))
        }
    }
}

pub fn build_forward_adapters(
    upstreams: &HashMap<String, ForwardUpstreamConfig>,
) -> HashMap<String, Arc<dyn UpstreamAdapter>> {
    upstreams
        .iter()
        .map(|(key, cfg)| (key.clone(), build_forward_adapter(key, cfg)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgecrab_core::ForwardAdapterKind;

    #[test]
    fn builds_hermes_auth_adapter_kind() {
        let mut upstreams = HashMap::new();
        upstreams.insert(
            "nous".into(),
            ForwardUpstreamConfig {
                base_url: "https://inference-api.nousresearch.com/v1".into(),
                adapter: ForwardAdapterKind::HermesAuth,
                auth_provider: Some("nous".into()),
                ..Default::default()
            },
        );
        let adapters = build_forward_adapters(&upstreams);
        assert_eq!(adapters["nous"].display_name(), "nous (hermes auth)");
    }

    #[test]
    fn builds_nous_portal_adapter_kind() {
        let mut upstreams = HashMap::new();
        upstreams.insert(
            "nous".into(),
            ForwardUpstreamConfig {
                base_url: "https://inference-api.nousresearch.com/v1".into(),
                adapter: ForwardAdapterKind::NousPortal,
                ..Default::default()
            },
        );
        let adapters = build_forward_adapters(&upstreams);
        assert_eq!(adapters["nous"].display_name(), "Nous Portal");
    }

    #[test]
    fn builds_xai_oauth_adapter_kind() {
        let mut upstreams = HashMap::new();
        upstreams.insert(
            "xai".into(),
            ForwardUpstreamConfig {
                base_url: "https://api.x.ai/v1".into(),
                adapter: ForwardAdapterKind::XaiOauth,
                auth_provider: Some("xai-oauth".into()),
                ..Default::default()
            },
        );
        let adapters = build_forward_adapters(&upstreams);
        assert_eq!(adapters["xai"].display_name(), "xAI Grok OAuth");
    }
}
