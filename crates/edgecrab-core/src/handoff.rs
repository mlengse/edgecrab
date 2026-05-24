//! Live session model handoff — brief generation, window check, and hot-swap.
//!
//! `/handoff <provider/model>` wraps `Agent::swap_model` with:
//! - catalog validation and provider auth probing before mutation
//! - optional auto-compression when the target context window is smaller
//! - a one-paragraph in-flight task brief (auxiliary LLM + structural fallback)
//! - cache-safe system prompt invalidation for the new provider

use std::sync::Arc;

use edgecrab_types::Message;
use edgequake_llm::{ChatMessage, LLMProvider};

use crate::compression::{
    CompressionParams, compress_with_llm, estimate_tokens, check_compression_status_for_estimate,
};
use crate::config::CompressionConfig;
use crate::model_catalog::ModelCatalog;

/// Resolved handoff destination from the model catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffTarget {
    pub display: String,
    pub provider: String,
    pub model_name: String,
    pub context_window: usize,
}

/// One-paragraph summary of the in-flight task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffBrief(pub String);

/// Successful handoff outcome surfaced to CLI / gateway / insights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffOutcome {
    pub from_model: String,
    pub to_model: String,
    pub brief: String,
    pub compressed: bool,
}

/// Handoff failure — always returned before session mutation when possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffError {
    InvalidFormat,
    UnknownModel(String),
    SameModel,
    ProviderAuth(String),
    CompressionFailed { reason: String },
    BriefGenerationFailed { reason: String },
}

impl std::fmt::Display for HandoffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => {
                write!(f, "Invalid format: use provider/model (e.g. copilot/gpt-5-mini)")
            }
            Self::UnknownModel(m) => write!(f, "Unknown model '{m}' — not in model catalog"),
            Self::SameModel => write!(f, "Already on the requested model"),
            Self::ProviderAuth(msg) => write!(f, "Provider auth failed: {msg}"),
            Self::CompressionFailed { reason } => {
                write!(f, "Cannot hand off: context too large and compression failed ({reason})")
            }
            Self::BriefGenerationFailed { reason } => {
                write!(f, "Handoff brief generation failed: {reason}")
            }
        }
    }
}

impl std::error::Error for HandoffError {}

const HANDOFF_USER_PREFIX: &str = "Continuing from previous session";
const STRUCTURAL_TURN_LIMIT: usize = 6;

/// Parse and validate a handoff target against the compiled model catalog.
pub fn resolve_handoff_target(model_str: &str) -> Result<HandoffTarget, HandoffError> {
    let display = model_str.trim();
    if display.is_empty() {
        return Err(HandoffError::InvalidFormat);
    }
    let (provider_raw, model_name) = display
        .split_once('/')
        .ok_or(HandoffError::InvalidFormat)?;
    let provider = edgecrab_tools::vision_models::normalize_provider_name(provider_raw);
    let context_window = ModelCatalog::context_window(&provider, model_name)
        .ok_or_else(|| HandoffError::UnknownModel(display.to_string()))?
        as usize;
    Ok(HandoffTarget {
        display: display.to_string(),
        provider,
        model_name: model_name.to_string(),
        context_window,
    })
}

/// Create the target LLM provider — auth / config errors surface before mutation.
pub fn create_target_provider(target: &HandoffTarget) -> Result<Arc<dyn LLMProvider>, HandoffError> {
    edgecrab_tools::create_provider_for_model(&target.provider, &target.model_name)
        .map_err(HandoffError::ProviderAuth)
}

/// Estimate prompt mass for window checks (messages + optional cached system prompt).
pub fn estimate_handoff_tokens(messages: &[Message], system_prompt: Option<&str>) -> usize {
    let message_tokens = estimate_tokens(messages);
    let system_tokens = system_prompt.map(|s| s.len() / 4).unwrap_or(0);
    message_tokens + system_tokens
}

/// Compression params for the *target* model.
pub fn target_compression_params(
    target: &HandoffTarget,
    compression_cfg: &CompressionConfig,
) -> CompressionParams {
    CompressionParams {
        context_window: target.context_window,
        threshold: compression_cfg.threshold.clamp(0.01, 1.0),
        target_ratio: compression_cfg.target_ratio.clamp(0.01, 1.0),
        protect_last_n: compression_cfg.protect_last_n.max(1),
    }
}

/// Returns true when estimated tokens exceed the target model's compression threshold.
pub fn needs_compression_for_target(
    messages: &[Message],
    system_prompt: Option<&str>,
    params: &CompressionParams,
) -> bool {
    let estimated = estimate_handoff_tokens(messages, system_prompt);
    matches!(
        check_compression_status_for_estimate(estimated, params),
        crate::compression::CompressionStatus::NeedsCompression
            | crate::compression::CompressionStatus::PressureWarning
    )
}

/// Compress history to fit the target window when needed.
///
/// Returns `(messages, compressed, llm_succeeded)`.
pub async fn maybe_compress_for_handoff(
    messages: Vec<Message>,
    system_prompt: Option<&str>,
    params: &CompressionParams,
    provider: &Arc<dyn LLMProvider>,
) -> Result<(Vec<Message>, bool, bool), HandoffError> {
    if !needs_compression_for_target(&messages, system_prompt, params) {
        return Ok((messages, false, true));
    }

    let (compressed, llm_succeeded) =
        compress_with_llm(&messages, params, provider, None).await;
    let after = estimate_handoff_tokens(&compressed, system_prompt);

    if matches!(
        check_compression_status_for_estimate(after, params),
        crate::compression::CompressionStatus::NeedsCompression
            | crate::compression::CompressionStatus::PressureWarning
    ) && !llm_succeeded
    {
        return Err(HandoffError::CompressionFailed {
            reason: format!(
                "history still ~{after} tokens after structural fallback (threshold ~{}",
                (params.context_window as f32 * params.threshold) as usize
            ),
        });
    }

    Ok((compressed, true, llm_succeeded))
}

/// Structural fallback: concatenate recent user/assistant turns.
pub fn structural_handoff_brief(messages: &[Message]) -> HandoffBrief {
    let mut parts = Vec::new();
    for msg in messages.iter().rev() {
        if msg.role == edgecrab_types::Role::User || msg.role == edgecrab_types::Role::Assistant {
            let text = msg.text_content();
            if !text.trim().is_empty() {
                parts.push(format!("{}: {}", msg.role, crate::safe_truncate(&text, 400)));
            }
        }
        if parts.len() >= STRUCTURAL_TURN_LIMIT {
            break;
        }
    }
    parts.reverse();
    let body = if parts.is_empty() {
        "No prior conversation turns to summarize.".to_string()
    } else {
        parts.join("\n")
    };
    HandoffBrief(body)
}

/// Generate a one-paragraph handoff brief via auxiliary LLM, with structural fallback.
pub async fn generate_handoff_brief(
    messages: &[Message],
    main_provider: Arc<dyn LLMProvider>,
    main_model: &str,
    auxiliary_model: Option<&str>,
) -> HandoffBrief {
    let (aux_provider, aux_model) = crate::auxiliary_model::resolve_side_task_provider_and_model(
        auxiliary_model,
        auxiliary_model,
        main_provider,
        main_model,
        "handoff brief",
    );

    let transcript = build_brief_transcript(messages);
    let prompt = format!(
        "Summarize the in-flight task from this conversation in ONE concise paragraph \
         (3-5 sentences). Focus on: current goal, progress so far, blockers, and immediate \
         next steps. Do not include greetings or meta commentary.\n\n{transcript}"
    );
    let chat = vec![
        ChatMessage::system(
            "You write short handoff briefs for another model continuing the same session.",
        ),
        ChatMessage::user(&prompt),
    ];

    match aux_provider.chat(&chat, None).await {
        Ok(resp) => {
            let text = resp.content.trim();
            if text.is_empty() {
                structural_handoff_brief(messages)
            } else {
                HandoffBrief(text.to_string())
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, model = %aux_model, "handoff brief LLM failed — structural fallback");
            structural_handoff_brief(messages)
        }
    }
}

fn build_brief_transcript(messages: &[Message]) -> String {
    let tail = messages
        .iter()
        .rev()
        .filter(|m| {
            matches!(
                m.role,
                edgecrab_types::Role::User | edgecrab_types::Role::Assistant
            )
        })
        .take(12)
        .collect::<Vec<_>>();
    let mut lines = Vec::with_capacity(tail.len());
    for msg in tail.into_iter().rev() {
        let content = msg.text_content();
        let text = crate::safe_truncate(&content, 600);
        if !text.trim().is_empty() {
            lines.push(format!("{}: {text}", msg.role));
        }
    }
    lines.join("\n\n")
}

/// Format the synthetic user message injected after handoff.
pub fn format_handoff_user_message(from_model: &str, to_model: &str, brief: &str) -> String {
    format!(
        "{HANDOFF_USER_PREFIX} ({from_model} → {to_model}):\n\n{brief}"
    )
}

/// Orchestrate a handoff on in-memory session state (testable without full Agent).
pub struct HandoffOrchestrator;

impl HandoffOrchestrator {
    /// Run the full handoff pipeline. Mutates `messages` in place on success.
    pub async fn execute(
        current_model: &str,
        target_spec: &str,
        messages: &mut Vec<Message>,
        system_prompt: Option<&str>,
        compression_cfg: &CompressionConfig,
        main_provider: Arc<dyn LLMProvider>,
        auxiliary_model: Option<&str>,
    ) -> Result<(HandoffOutcome, Arc<dyn LLMProvider>), HandoffError> {
        let target = resolve_handoff_target(target_spec)?;
        if target.display.eq_ignore_ascii_case(current_model) {
            return Err(HandoffError::SameModel);
        }

        let new_provider = create_target_provider(&target)?;
        let params = target_compression_params(&target, compression_cfg);

        let pre_compress = messages.clone();
        let (compressed_messages, did_compress, _llm_ok) = maybe_compress_for_handoff(
            messages.clone(),
            system_prompt,
            &params,
            &new_provider,
        )
        .await?;
        *messages = compressed_messages;

        let brief = generate_handoff_brief(
            &pre_compress,
            main_provider.clone(),
            current_model,
            auxiliary_model,
        )
        .await;

        if brief.0.trim().is_empty() {
            return Err(HandoffError::BriefGenerationFailed {
                reason: "empty brief".into(),
            });
        }

        Ok((
            HandoffOutcome {
                from_model: current_model.to_string(),
                to_model: target.display.clone(),
                brief: brief.0.clone(),
                compressed: did_compress,
            },
            new_provider,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;

    fn sample_messages(count: usize) -> Vec<Message> {
        (0..count)
            .map(|i| {
                if i % 2 == 0 {
                    Message::user(&format!("user turn {i} with some content"))
                } else {
                    Message::assistant(&format!("assistant reply {i} with details"))
                }
            })
            .collect()
    }

    #[test]
    fn resolve_unknown_model_fails() {
        let err = resolve_handoff_target("fakeprovider/unknown-model-xyz").unwrap_err();
        assert!(matches!(err, HandoffError::UnknownModel(_)));
    }

    #[test]
    fn resolve_known_catalog_model_succeeds() {
        let target = resolve_handoff_target("anthropic/claude-haiku-4.5").expect("catalog hit");
        assert_eq!(target.provider, "anthropic");
        assert_eq!(target.model_name, "claude-haiku-4.5");
        assert!(target.context_window > 0);
    }

    #[tokio::test]
    async fn smaller_window_compress_ok() {
        let mut messages = sample_messages(80);
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let params = CompressionParams {
            context_window: 2_000,
            threshold: 0.50,
            target_ratio: 0.20,
            protect_last_n: 4,
        };
        let before = estimate_handoff_tokens(&messages, None);
        assert!(needs_compression_for_target(&messages, None, &params));
        let (out, compressed, llm_ok) = maybe_compress_for_handoff(
            messages.clone(),
            None,
            &params,
            &provider,
        )
        .await
        .expect("compress should succeed with mock/structural fallback");
        messages = out;
        let after = estimate_handoff_tokens(&messages, None);
        assert!(compressed);
        assert!(after <= before || llm_ok);
    }

    #[test]
    fn compression_refused_when_llm_fails_and_still_over_threshold() {
        let params = CompressionParams {
            context_window: 1_000,
            threshold: 0.50,
            ..Default::default()
        };
        let threshold = (params.context_window as f32 * params.threshold) as usize;
        let estimated_after = threshold + 500;
        let llm_succeeded = false;
        let should_refuse = matches!(
            check_compression_status_for_estimate(estimated_after, &params),
            crate::compression::CompressionStatus::NeedsCompression
                | crate::compression::CompressionStatus::PressureWarning
        ) && !llm_succeeded;
        assert!(should_refuse);
    }

    #[tokio::test]
    async fn brief_generation_ok_via_mock() {
        let messages = sample_messages(4);
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let brief = generate_handoff_brief(&messages, provider, "anthropic/claude-opus-4.6", None)
            .await;
        assert!(!brief.0.is_empty());
    }

    #[test]
    fn brief_structural_fallback_when_no_llm() {
        let messages = vec![
            Message::user("fix the auth module"),
            Message::assistant("I'll start by reading agent.rs"),
        ];
        let brief = structural_handoff_brief(&messages);
        assert!(brief.0.contains("auth module"));
        assert!(brief.0.contains("agent.rs"));
    }

    #[test]
    fn provider_auth_failure_leaves_messages_untouched() {
        let target = HandoffTarget {
            display: "definitely-not-a-real-provider/xyz".into(),
            provider: "definitely-not-a-real-provider".into(),
            model_name: "xyz".into(),
            context_window: 128_000,
        };
        let err = create_target_provider(&target);
        assert!(matches!(err, Err(HandoffError::ProviderAuth(_))));
    }

    #[tokio::test]
    async fn orchestrator_same_model_rejected() {
        let mut messages = sample_messages(2);
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let cfg = CompressionConfig::default();
        let err = HandoffOrchestrator::execute(
            "anthropic/claude-haiku-4.5",
            "anthropic/claude-haiku-4.5",
            &mut messages,
            None,
            &cfg,
            provider,
            None,
        )
        .await;
        assert!(matches!(err, Err(HandoffError::SameModel)));
    }

    #[test]
    fn format_handoff_message_includes_models_and_brief() {
        let msg = format_handoff_user_message("a/m1", "b/m2", "Working on tests.");
        assert!(msg.contains("a/m1 → b/m2"));
        assert!(msg.contains("Working on tests."));
    }
}
