//! Fallback orchestrator — tries backends in order on transient failures.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use super::backend::{SearchResult, WebSearchBackend};
use super::backend_settings::{backend_is_configured, resolve_timeout_secs};
use super::config::{ResolvedChain, SearchOptions};
use super::error::{ChainFailureSummary, SearchError};
use super::rate_limit::RateLimiter;
use super::registry::get_web_search_backend;

static GLOBAL_RATE_LIMITER: OnceLock<Arc<RateLimiter>> = OnceLock::new();

fn global_rate_limiter() -> Arc<RateLimiter> {
    GLOBAL_RATE_LIMITER
        .get_or_init(|| Arc::new(RateLimiter::new()))
        .clone()
}

/// Ordered backend chain with rate limiting and fallback policy.
pub struct BackendChain {
    resolved: ResolvedChain,
    backends: Vec<(String, Arc<dyn WebSearchBackend>)>,
    rate_limiter: Arc<RateLimiter>,
    default_timeout_secs: u64,
}

impl BackendChain {
    pub fn from_resolved(resolved: &ResolvedChain) -> Result<Self, SearchError> {
        let rate_limiter = global_rate_limiter();
        let mut backends = Vec::new();
        for name in &resolved.names {
            let cfg = resolved.backend_config(name);
            if !backend_is_configured(name, &cfg) {
                tracing::debug!(
                    backend = %name,
                    "web_search: skipping backend — credentials not configured"
                );
                continue;
            }
            let backend = get_web_search_backend(name).ok_or_else(|| {
                SearchError::hard(
                    "web_search",
                    format!("Unknown web search backend '{name}'."),
                )
            })?;
            if let Some(rps) = cfg.rps {
                rate_limiter.configure(name, Some(rps));
            }
            backends.push((name.clone(), backend));
        }
        if backends.is_empty() {
            if let Some(backend) = get_web_search_backend("ddgs") {
                backends.push(("ddgs".into(), backend));
            } else {
                return Err(SearchError::hard(
                    "web_search",
                    "No web search backends configured.",
                ));
            }
        }
        Ok(Self {
            resolved: resolved.clone(),
            backends,
            rate_limiter,
            default_timeout_secs: resolved.config.timeout_secs,
        })
    }

    pub async fn search(
        &self,
        query: &str,
        mut opts: SearchOptions,
    ) -> Result<(Vec<SearchResult>, String), SearchError> {
        if opts.timeout_secs == 0 {
            opts.timeout_secs = self.default_timeout_secs;
        }
        let mut attempts: Vec<(String, String)> = Vec::new();

        for (name, backend) in &self.backends {
            if !self.rate_limiter.is_available(name) {
                attempts.push((name.clone(), "rate limit budget exhausted".into()));
                continue;
            }

            let backend_cfg = self.resolved.backend_config(name);
            opts.backend_config = backend_cfg.clone();
            opts.timeout_secs = resolve_timeout_secs(&backend_cfg, opts.timeout_secs);

            if !backend_is_configured(name, &backend_cfg) {
                continue;
            }

            match backend.search(query, &opts).await {
                Ok(results) => return Ok((results, backend.name().to_string())),
                Err(err) if err.is_fallback_eligible() => {
                    tracing::warn!(
                        backend = %name,
                        error = %err,
                        "web_search: backend unavailable, trying next in chain"
                    );
                    // Only cooldown paid APIs with explicit RPS buckets — not ddgs scrape failures.
                    if matches!(err.kind, super::error::SearchErrorKind::RateLimit)
                        && self.rate_limiter.has_rps_bucket(name)
                    {
                        self.rate_limiter
                            .mark_rate_limited(name, Duration::from_secs(30));
                    }
                    attempts.push((name.clone(), err.message));
                }
                Err(err) => return Err(err),
            }
        }

        let summary = ChainFailureSummary { attempts };
        Err(SearchError::hard("web_search", summary.user_message()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_failure_user_message_is_concise() {
        let summary = ChainFailureSummary {
            attempts: vec![
                ("searxng".into(), "SEARXNG_URL is not set".into()),
                ("ddgs".into(), "timeout".into()),
            ],
        };
        let msg = summary.user_message();
        assert!(msg.contains("searxng → ddgs"));
        assert!(!msg.contains("All web search backends failed"));
    }
}
