//! Prompt prefix cache eligibility — single source of truth (Hermes parity).
//!
//! Mirrors `hermes-agent/agent/agent_runtime_helpers.py::anthropic_prompt_cache_policy`.
//! EdgeCrab's two-block stable/dynamic split only helps when the provider honours
//! Anthropic-style `cache_control` markers.

/// Whether prompt caching should be active for this provider/model pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PromptCacheDecision {
    pub should_cache: bool,
    /// When true, markers belong on inner content blocks (native Anthropic).
    /// When false, envelope layout (OpenRouter / Qwen on OpenAI wire).
    pub native_inner_layout: bool,
}

/// Host suffix match — `host` ends with `.suffix` or equals `suffix`.
fn host_matches(base_url: Option<&str>, suffix: &str) -> bool {
    let Some(url) = base_url else {
        return false;
    };
    let host = url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    host == suffix || host.ends_with(&format!(".{suffix}"))
}

/// Decide whether EdgeCrab should attach prompt cache breakpoints.
pub fn decide_prompt_cache(
    provider_name: &str,
    model: &str,
    base_url: Option<&str>,
) -> PromptCacheDecision {
    let provider_lower = provider_name.to_ascii_lowercase();
    let model_lower = model.to_ascii_lowercase();
    let is_claude = model_lower.contains("claude");
    let is_openrouter = host_matches(base_url, "openrouter.ai");
    let is_nous_portal = base_url
        .map(|u| u.to_ascii_lowercase().contains("nousresearch"))
        .unwrap_or(false);
    let is_native_anthropic = provider_lower == "anthropic"
        || host_matches(base_url, "api.anthropic.com");

    if is_native_anthropic {
        return PromptCacheDecision {
            should_cache: true,
            native_inner_layout: true,
        };
    }
    if (is_openrouter || is_nous_portal) && is_claude {
        return PromptCacheDecision {
            should_cache: true,
            native_inner_layout: false,
        };
    }
    if is_nous_portal && model_lower.contains("qwen") {
        return PromptCacheDecision {
            should_cache: true,
            native_inner_layout: false,
        };
    }
    if is_claude && provider_lower.contains("anthropic") {
        return PromptCacheDecision {
            should_cache: true,
            native_inner_layout: true,
        };
    }

    let is_minimax = matches!(provider_lower.as_str(), "minimax" | "minimax-cn")
        || host_matches(base_url, "api.minimax.io")
        || host_matches(base_url, "api.minimaxi.com");
    if is_claude && is_minimax {
        return PromptCacheDecision {
            should_cache: true,
            native_inner_layout: true,
        };
    }

    let model_is_qwen = model_lower.contains("qwen");
    let provider_is_alibaba_family = matches!(
        provider_lower.as_str(),
        "opencode" | "opencode-zen" | "opencode-go" | "alibaba"
    );
    if provider_is_alibaba_family && model_is_qwen {
        return PromptCacheDecision {
            should_cache: true,
            native_inner_layout: false,
        };
    }

    // Legacy gate: direct Anthropic provider id without URL (tests, mocks).
    if provider_lower == "anthropic" {
        return PromptCacheDecision {
            should_cache: true,
            native_inner_layout: true,
        };
    }

    PromptCacheDecision::default()
}

/// Fast check used by the conversation loop before building cache config.
pub fn provider_supports_prompt_caching(
    provider_name: &str,
    model: &str,
    base_url: Option<&str>,
) -> bool {
    decide_prompt_cache(provider_name, model, base_url).should_cache
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_anthropic_enables_cache() {
        let d = decide_prompt_cache(
            "anthropic",
            "claude-sonnet-4-20250514",
            Some("https://api.anthropic.com"),
        );
        assert!(d.should_cache);
        assert!(d.native_inner_layout);
    }

    #[test]
    fn openrouter_claude_uses_envelope_layout() {
        let d = decide_prompt_cache(
            "openrouter",
            "anthropic/claude-sonnet-4",
            Some("https://openrouter.ai/api/v1"),
        );
        assert!(d.should_cache);
        assert!(!d.native_inner_layout);
    }

    #[test]
    fn nous_portal_qwen_enables_cache() {
        let d = decide_prompt_cache(
            "nous",
            "qwen3.6-plus",
            Some("https://api.nousresearch.com/v1"),
        );
        assert!(d.should_cache);
        assert!(!d.native_inner_layout);
    }

    #[test]
    fn opencode_qwen_enables_cache() {
        let d = decide_prompt_cache("opencode-go", "qwen3-coder", None);
        assert!(d.should_cache);
    }

    #[test]
    fn gpt4o_disables_cache() {
        let d = decide_prompt_cache("openai", "gpt-4o", None);
        assert!(!d.should_cache);
    }
}
