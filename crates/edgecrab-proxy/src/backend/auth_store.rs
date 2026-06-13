//! Read inference credentials from Hermes-format `auth.json` (bridge until 024 OAuth in Rust).

use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::Value;

use async_trait::async_trait;

use super::adapter::{UpstreamAdapter, UpstreamCredential};
use super::auth_file::bearer_and_base_from_entry;
use crate::error::ProxyError;

const DEFAULT_INFERENCE_FALLBACK: &str = "https://inference-api.nousresearch.com/v1";

/// Adapter that loads `agent_key` from `~/.hermes/auth.json` or `~/.edgecrab/auth.json`.
pub struct HermesAuthFileAdapter {
    name: String,
    display_name: String,
    auth_provider: String,
    auth_path: PathBuf,
    fallback_base_url: String,
    paths: HashSet<String>,
    auth_hint: String,
}

impl HermesAuthFileAdapter {
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
        let hint = auth_hint.unwrap_or_else(|| {
            format!("run `hermes auth add {provider}` or configure bearer_env for '{upstream_key}'")
        });
        Self {
            name: upstream_key.to_string(),
            display_name: format!("{upstream_key} (hermes auth)"),
            auth_provider: provider,
            auth_path: auth_path.unwrap_or_else(super::auth_file::default_auth_path),
            fallback_base_url: fallback_base_url.trim_end_matches('/').to_string(),
            paths: HashSet::from([
                "/chat/completions".into(),
                "/completions".into(),
                "/embeddings".into(),
                "/models".into(),
            ]),
            auth_hint: hint,
        }
    }

    fn read_provider_state(&self) -> Result<Value, ProxyError> {
        super::auth_file::read_provider_state(&self.auth_path, &self.auth_provider)
    }

    fn inference_fallback(&self) -> &str {
        if self.fallback_base_url.is_empty() {
            DEFAULT_INFERENCE_FALLBACK
        } else {
            &self.fallback_base_url
        }
    }
}

/// Extract provider object from Hermes auth store (`providers` or legacy `systems`).
pub fn provider_state_from_doc(doc: &Value, provider: &str) -> Option<Value> {
    if let Some(p) = doc.get("providers").and_then(|m| m.get(provider)) {
        return Some(p.clone());
    }
    let legacy = match provider {
        "nous" => "nous_portal",
        other => other,
    };
    doc.get("systems")?.get(legacy).cloned()
}

#[async_trait]
impl UpstreamAdapter for HermesAuthFileAdapter {
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
        let fallback = self.inference_fallback();
        self.read_provider_state()
            .ok()
            .and_then(|s| bearer_and_base_from_entry(&s, fallback))
            .is_some()
    }

    async fn get_credential(&self) -> Result<UpstreamCredential, ProxyError> {
        let state = self.read_provider_state()?;
        let fallback = self.inference_fallback();
        let (bearer, base_url) = bearer_and_base_from_entry(&state, fallback).ok_or_else(|| {
            ProxyError::UpstreamAuth(format!(
                "{}: missing bearer (agent_key / access_token) in {}",
                self.display_name,
                self.auth_path.display()
            ))
        })?;
        Ok(UpstreamCredential {
            bearer,
            base_url,
            token_type: "Bearer".into(),
        })
    }

    fn auth_hint(&self) -> String {
        self.auth_hint.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_providers_nous_agent_key() {
        let doc: Value = serde_json::json!({
            "providers": {
                "nous": {
                    "agent_key": "jwt-abc",
                    "base_url": "https://inference.example/v1"
                }
            }
        });
        let state = provider_state_from_doc(&doc, "nous").expect("nous");
        let key = state["agent_key"].as_str().expect("key");
        assert_eq!(key, "jwt-abc");
    }

    #[test]
    fn hermes_auth_reads_tokens_access_token() {
        let doc: Value = serde_json::json!({
            "providers": {
                "xai-oauth": {
                    "tokens": { "access_token": "xai-bearer" },
                    "base_url": "https://api.x.ai/v1"
                }
            }
        });
        let state = provider_state_from_doc(&doc, "xai-oauth").expect("xai");
        let (bearer, base) =
            bearer_and_base_from_entry(&state, "https://fallback/v1").expect("pair");
        assert_eq!(bearer, "xai-bearer");
        assert_eq!(base, "https://api.x.ai/v1");
    }

    #[test]
    fn parses_legacy_systems_nous_portal() {
        let doc: Value = serde_json::json!({
            "systems": {
                "nous_portal": { "agent_key": "legacy-jwt" }
            }
        });
        let state = provider_state_from_doc(&doc, "nous").expect("nous");
        assert_eq!(state["agent_key"], "legacy-jwt");
    }
}
