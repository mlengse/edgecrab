//! Subscription OAuth primitives (spec 024).
//!
//! Provider-specific login flows live in `edgecrab-proxy` (forwarder auth.json)
//! or `edgequake_llm` (Copilot GitHub device flow). This module holds shared
//! crypto and provider identifiers for the model router phase.

pub mod pkce;

/// Hermes-compatible provider id for xAI Grok OAuth.
pub const XAI_OAUTH_PROVIDER: &str = "xai-oauth";

/// Hermes-compatible provider id for Nous Portal.
pub const NOUS_PROVIDER: &str = "nous";

/// CLI-facing aliases that resolve to [`XAI_OAUTH_PROVIDER`].
pub const XAI_OAUTH_ALIASES: &[&str] = &[
    "grok",
    "grok-oauth",
    "x-ai-oauth",
    "xai-grok-oauth",
    "super-grok",
    "super_grok",
];

pub fn is_xai_oauth_alias(target: &str) -> bool {
    let t = target.to_ascii_lowercase();
    t == XAI_OAUTH_PROVIDER || XAI_OAUTH_ALIASES.contains(&t.as_str())
}
