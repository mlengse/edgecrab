//! Resolve OpenAI `model` field → provider bridge (Mode B) or forward upstream (Mode A).

use std::collections::HashMap;
use std::sync::Arc;

use edgecrab_core::model_catalog::ModelCatalog;
use edgecrab_core::ForwardUpstreamConfig;
use edgecrab_tools::create_provider_for_model;
use edgequake_llm::LLMProvider;

use crate::backend::adapter::UpstreamAdapter;
use crate::backend::factory::build_forward_adapters as build_adapters;
use crate::error::ProxyError;

pub const FORWARD_SPEC_PREFIX: &str = "forward:";

#[derive(Debug, Clone)]
pub struct ResolvedBackend {
    pub display_model: String,
    pub runtime_provider: String,
    pub model_name: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedForward {
    pub display_model: String,
    pub upstream_key: String,
}

#[derive(Debug, Clone)]
pub enum ResolvedRoute {
    Provider(ResolvedBackend),
    Forward(ResolvedForward),
}

/// Build upstream adapters from config (OAuth adapters register in `backend/factory.rs`).
pub fn build_forward_adapters(
    upstreams: &HashMap<String, ForwardUpstreamConfig>,
) -> HashMap<String, Arc<dyn UpstreamAdapter>> {
    build_adapters(upstreams)
}

fn resolve_spec_string(
    requested: &str,
    aliases: &HashMap<String, String>,
    default_spec: Option<&str>,
) -> Result<(String, String), ProxyError> {
    let spec = aliases
        .get(requested)
        .cloned()
        .or_else(|| {
            if requested.contains('/') || requested.starts_with(FORWARD_SPEC_PREFIX) {
                Some(requested.to_string())
            } else {
                default_spec.map(str::to_string)
            }
        })
        .ok_or_else(|| {
            ProxyError::ModelNotFound(format!(
                "unknown model '{requested}'. Configure proxy.model_aliases, forward:nous, or provider/model."
            ))
        })?;
    Ok((requested.to_string(), spec))
}

pub fn resolve_route(
    requested: &str,
    aliases: &HashMap<String, String>,
    default_spec: Option<&str>,
    forward_upstreams: &HashMap<String, ForwardUpstreamConfig>,
) -> Result<ResolvedRoute, ProxyError> {
    let (display_model, spec) = resolve_spec_string(requested, aliases, default_spec)?;

    if let Some(key) = spec.strip_prefix(FORWARD_SPEC_PREFIX) {
        let key = key.trim();
        if key.is_empty() {
            return Err(ProxyError::ModelNotFound(
                "forward: requires an upstream key (e.g. forward:nous)".into(),
            ));
        }
        if !forward_upstreams.contains_key(key) {
            return Err(ProxyError::ModelNotFound(format!(
                "forward upstream '{key}' not configured in proxy.forward_upstreams"
            )));
        }
        return Ok(ResolvedRoute::Forward(ResolvedForward {
            display_model,
            upstream_key: key.to_string(),
        }));
    }

    let backend = ModelCatalog::resolve_spec_lenient(&spec).ok_or_else(|| {
        ProxyError::ModelNotFound(format!(
            "invalid model spec '{spec}' (expected provider/model or forward:name)"
        ))
    })?;

    Ok(ResolvedRoute::Provider(ResolvedBackend {
        display_model,
        runtime_provider: backend.runtime_provider,
        model_name: backend.model_name,
    }))
}

pub fn resolve_model(
    requested: &str,
    aliases: &HashMap<String, String>,
    default_spec: Option<&str>,
) -> Result<ResolvedBackend, ProxyError> {
    match resolve_route(requested, aliases, default_spec, &HashMap::new())? {
        ResolvedRoute::Provider(b) => Ok(b),
        ResolvedRoute::Forward(_) => Err(ProxyError::BadRequest(
            "model uses forward: upstream; use provider bridge config or forward alias".into(),
        )),
    }
}

pub fn create_provider(backend: &ResolvedBackend) -> Result<Arc<dyn LLMProvider>, ProxyError> {
    create_provider_for_model(&backend.runtime_provider, &backend.model_name)
        .map_err(ProxyError::Upstream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgecrab_core::ForwardUpstreamConfig;

    #[test]
    fn resolves_direct_mock_spec() {
        let r = resolve_model("mock/test-model", &HashMap::new(), None).expect("resolve");
        assert_eq!(r.runtime_provider, "mock");
        assert_eq!(r.model_name, "test-model");
    }

    #[test]
    fn resolves_discovered_lmstudio_model() {
        let r = resolve_model("lmstudio/liquid/lfm2.5-1.2b", &HashMap::new(), None)
            .expect("resolve dynamic lmstudio model");
        assert_eq!(r.runtime_provider, "lmstudio");
        assert_eq!(r.model_name, "liquid/lfm2.5-1.2b");
    }

    #[test]
    fn unknown_model_without_alias_returns_not_found() {
        let err = resolve_model("no-such-model", &HashMap::new(), None).unwrap_err();
        assert!(matches!(err, ProxyError::ModelNotFound(_)));
    }

    #[test]
    fn resolves_via_alias() {
        let mut aliases = HashMap::new();
        aliases.insert("edgecrab-mock".into(), "mock/test-model".into());
        let r = resolve_model("edgecrab-mock", &aliases, None).expect("resolve");
        assert_eq!(r.runtime_provider, "mock");
    }

    #[test]
    fn resolves_forward_upstream() {
        let mut upstreams = HashMap::new();
        upstreams.insert(
            "test-up".into(),
            ForwardUpstreamConfig {
                base_url: "http://127.0.0.1:9/v1".into(),
                bearer: Some("up-secret".into()),
                ..Default::default()
            },
        );
        let route = resolve_route("forward:test-up", &HashMap::new(), None, &upstreams)
            .expect("route");
        assert!(matches!(
            route,
            ResolvedRoute::Forward(ResolvedForward {
                upstream_key,
                ..
            }) if upstream_key == "test-up"
        ));
    }
}
