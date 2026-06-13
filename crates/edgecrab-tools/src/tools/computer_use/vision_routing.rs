//! Vision-routing policy for `computer_use` captures (mirrors Hermes `vision_routing.py`).

use crate::config_ref::AppConfigRef;
use crate::vision_models::{
    model_supports_vision, normalize_provider_name, parse_provider_model_spec,
};

/// True when the user explicitly configured an auxiliary vision backend.
pub fn explicit_aux_vision_override(config: &AppConfigRef) -> bool {
    let provider = config
        .auxiliary_provider
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let model = config.auxiliary_model.as_deref().unwrap_or("").trim();
    let base_url = config.auxiliary_base_url.as_deref().unwrap_or("").trim();

    !(provider.is_empty() || provider == "auto") || !model.is_empty() || !base_url.is_empty()
}

/// Whether *provider* accepts image content inside tool-result messages.
///
/// Mirrors Hermes `tools.vision_tools._supports_media_in_tool_results`.
pub fn provider_accepts_multimodal_tool_result(provider: &str, model: &str) -> Option<bool> {
    let p = normalize_provider_name(provider);
    if p.is_empty() {
        return None;
    }

    const AGGREGATORS: &[&str] = &[
        "openrouter",
        "nous",
        "vertexai",
        "bedrock",
        "anthropic-vertex",
        "google-vertex",
    ];
    if AGGREGATORS.contains(&p.as_str()) {
        return Some(true);
    }

    if matches!(
        p.as_str(),
        "anthropic"
            | "claude"
            | "anthropic-direct"
            | "openai"
            | "openai-chat"
            | "openai-codex"
            | "azure"
    ) {
        return Some(true);
    }

    if matches!(
        p.as_str(),
        "gemini" | "google" | "google-gemini" | "google-vertex-gemini"
    ) {
        let lowered = model.trim().to_ascii_lowercase();
        return Some(
            lowered.contains("gemini-3")
                || lowered.contains("gemini-pro-3")
                || lowered.contains("gemini-flash-3"),
        );
    }

    if p == "vscode-copilot" {
        // Copilot enforces strict context limits; inline tool-result images blow the window.
        return Some(false);
    }

    Some(false)
}

/// Return true when a capture screenshot should be pre-analysed via auxiliary vision.
pub fn should_route_capture_to_aux_vision(
    provider: &str,
    model: &str,
    config: &AppConfigRef,
) -> bool {
    if explicit_aux_vision_override(config) {
        return true;
    }

    match provider_accepts_multimodal_tool_result(provider, model) {
        None | Some(false) => return true,
        Some(true) => {}
    }

    !model_supports_vision(None, provider, model)
}

/// Parse the active session model (`provider/model`) from config.
pub fn active_provider_model(config: &AppConfigRef) -> (String, String) {
    if let Some((provider, model)) = parse_provider_model_spec(&config.active_model) {
        return (provider, model);
    }
    ("unknown".into(), config.active_model.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_aux_override_detects_model_only() {
        let cfg = AppConfigRef {
            auxiliary_model: Some("openai/gpt-4o".into()),
            ..Default::default()
        };
        assert!(explicit_aux_vision_override(&cfg));
    }

    #[test]
    fn explicit_aux_override_ignores_auto_provider() {
        let cfg = AppConfigRef::default();
        assert!(!explicit_aux_vision_override(&cfg));
    }

    #[test]
    fn route_to_aux_when_main_model_not_vision() {
        let cfg = AppConfigRef::default();
        assert!(should_route_capture_to_aux_vision(
            "openai",
            "gpt-3.5-turbo",
            &cfg
        ));
    }

    #[test]
    fn keep_multimodal_for_vision_main_model() {
        let cfg = AppConfigRef::default();
        assert!(!should_route_capture_to_aux_vision(
            "anthropic",
            "claude-opus-4.6",
            &cfg
        ));
    }

    #[test]
    fn explicit_aux_forces_routing_even_for_vision_main() {
        let cfg = AppConfigRef {
            auxiliary_provider: Some("openai".into()),
            auxiliary_model: Some("gpt-4o".into()),
            ..Default::default()
        };
        assert!(should_route_capture_to_aux_vision(
            "anthropic",
            "claude-opus-4.6",
            &cfg
        ));
    }

    #[test]
    fn copilot_routes_to_aux_or_text_only() {
        let cfg = AppConfigRef::default();
        assert!(should_route_capture_to_aux_vision(
            "copilot",
            "gpt-5-mini",
            &cfg
        ));
    }

    #[test]
    fn anthropic_accepts_tool_images() {
        assert_eq!(
            provider_accepts_multimodal_tool_result("anthropic", "claude-opus-4.6"),
            Some(true)
        );
    }

    #[test]
    fn gemini_3_accepts_tool_images() {
        assert_eq!(
            provider_accepts_multimodal_tool_result("gemini", "gemini-3-pro-preview"),
            Some(true)
        );
        assert_eq!(
            provider_accepts_multimodal_tool_result("google", "gemini-flash-3"),
            Some(true)
        );
    }

    #[test]
    fn gemini_2_does_not_accept_tool_images() {
        assert_eq!(
            provider_accepts_multimodal_tool_result("gemini", "gemini-2.0-flash"),
            Some(false)
        );
    }

    #[test]
    fn provider_rejects_multimodal_tool_results_routes_to_aux() {
        let cfg = AppConfigRef::default();
        assert!(
            should_route_capture_to_aux_vision("custom-provider", "vision-model", &cfg)
                || should_route_capture_to_aux_vision("openai", "gpt-3.5-turbo", &cfg)
        );
        assert!(should_route_capture_to_aux_vision(
            "copilot", "gpt-4.1", &cfg
        ));
    }

    #[test]
    fn openrouter_aggregator_accepts_tool_images() {
        assert_eq!(
            provider_accepts_multimodal_tool_result("openrouter", "anthropic/claude-sonnet-4"),
            Some(true)
        );
    }
}
