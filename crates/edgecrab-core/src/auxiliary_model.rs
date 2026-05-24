//! Shared routing for auxiliary / side-task LLM calls (goal judge, shadow judge, …).
//!
//! Priority: explicit task model → `auxiliary.model` → main session model.

use std::sync::Arc;

use edgequake_llm::LLMProvider;

/// Resolve `(provider, model_string)` for a side-task LLM call.
///
/// When the chosen model string contains `/`, the prefix is treated as the
/// provider family and a new provider is created via
/// `edgecrab_tools::create_provider_for_model`. On failure, falls back to
/// the main provider with the raw model string.
pub fn resolve_side_task_provider_and_model(
    preferred_model: Option<&str>,
    fallback_auxiliary_model: Option<&str>,
    main_provider: Arc<dyn LLMProvider>,
    main_model: &str,
    log_label: &str,
) -> (Arc<dyn LLMProvider>, String) {
    let candidate = preferred_model
        .or(fallback_auxiliary_model)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let Some(raw_model) = candidate else {
        return (main_provider, main_model.to_string());
    };

    if let Some((provider_name, model_name)) = raw_model.split_once('/') {
        let canonical = edgecrab_tools::vision_models::normalize_provider_name(provider_name);
        match edgecrab_tools::create_provider_for_model(&canonical, model_name) {
            Ok(p) => return (p, raw_model.to_string()),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    raw_model,
                    log_label,
                    "side-task model: failed to create configured provider, using main provider"
                );
            }
        }
    }

    (main_provider, raw_model.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;

    #[test]
    fn falls_back_to_main_when_no_override() {
        let main = Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>;
        let (p, m) = resolve_side_task_provider_and_model(None, None, main.clone(), "main/m", "test");
        assert!(Arc::ptr_eq(&p, &main));
        assert_eq!(m, "main/m");
    }

    #[test]
    fn prefers_explicit_over_auxiliary() {
        let main = Arc::new(MockProvider::new()) as Arc<dyn LLMProvider>;
        let (_, m) = resolve_side_task_provider_and_model(
            Some("cheap/model"),
            Some("aux/model"),
            main,
            "main/m",
            "test",
        );
        assert_eq!(m, "cheap/model");
    }
}
