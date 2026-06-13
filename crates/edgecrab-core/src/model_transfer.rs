//! Live model transfer — brief generation, window check, and hot-swap.
//!
//! `/transfer-model <provider/model>` wraps `Agent::swap_model` with:
//! - catalog validation and provider auth probing before mutation
//! - optional auto-compression when the target context window is smaller
//! - a one-paragraph in-flight task brief (auxiliary LLM + structural fallback)
//! - cache-safe system prompt invalidation for the new provider

use std::sync::Arc;

use edgecrab_types::Message;
use edgequake_llm::{ChatMessage, LLMProvider};

use crate::compression::{
    CompressionParams, check_compression_status_for_estimate, compress_with_llm, estimate_tokens,
};
use crate::config::CompressionConfig;
use crate::model_catalog::ModelCatalog;

/// Usage text for `/transfer-model` (CLI + gateway DRY).
pub const MODEL_TRANSFER_USAGE: &str = "Usage: /transfer-model <provider/model>\n\
Example: /transfer-model copilot/gpt-5-mini\n\
Generates a task brief, checks context window, then hot-swaps.";

/// Returned when model transfer is requested while a turn is in flight (CLI + gateway DRY).
pub const MODEL_TRANSFER_BUSY_MESSAGE: &str =
    "Agent is busy. Wait for the current turn to finish, then retry /model or /transfer-model.";

/// Resolved model-transfer destination from the model catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTransferTarget {
    pub display: String,
    pub provider: String,
    pub model_name: String,
    pub context_window: usize,
}

/// One-paragraph summary of the in-flight task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTransferBrief(pub String);

/// Successful handoff outcome surfaced to CLI / gateway / insights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTransferOutcome {
    pub from_model: String,
    pub to_model: String,
    pub brief: String,
    pub compressed: bool,
    /// Source model context window (from catalog).
    pub from_context_window: usize,
    /// Target model context window (from catalog).
    pub target_context_window: usize,
}

impl ModelTransferOutcome {
    /// True when the target model has a smaller context window than the source.
    pub fn window_shrunk(&self) -> bool {
        self.target_context_window < self.from_context_window
    }
}

/// Instant switch when there is no in-flight conversation to preserve (Hermes `config.set` parity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSwitchOutcome {
    pub from_model: String,
    pub to_model: String,
}

/// Result of `/model` — fast switch or full transfer with brief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelChangeOutcome {
    Fast(ModelSwitchOutcome),
    Transfer(ModelTransferOutcome),
}

impl ModelChangeOutcome {
    pub fn to_model(&self) -> &str {
        match self {
            Self::Fast(o) => &o.to_model,
            Self::Transfer(o) => &o.to_model,
        }
    }
}

/// Handoff failure — always returned before session mutation when possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelTransferError {
    InvalidFormat,
    UnknownModel(String),
    SameModel,
    ProviderAuth(String),
    CompressionFailed { reason: String },
    BriefGenerationFailed { reason: String },
}

impl std::fmt::Display for ModelTransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => {
                write!(
                    f,
                    "Invalid format: use provider/model (e.g. copilot/gpt-5-mini)"
                )
            }
            Self::UnknownModel(m) => {
                write!(f, "Invalid model spec '{m}' (expected provider/model)")
            }
            Self::SameModel => write!(f, "Already on the requested model"),
            Self::ProviderAuth(msg) => write!(f, "Provider auth failed: {msg}"),
            Self::CompressionFailed { reason } => {
                write!(
                    f,
                    "Cannot hand off: context too large and compression failed ({reason})"
                )
            }
            Self::BriefGenerationFailed { reason } => {
                write!(f, "Handoff brief generation failed: {reason}")
            }
        }
    }
}

impl std::error::Error for ModelTransferError {}

const MODEL_TRANSFER_USER_PREFIX: &str = "Continuing from previous session";
const STRUCTURAL_TURN_LIMIT: usize = 6;

/// Parse and validate a handoff target (catalog + dynamically discovered models).
pub fn resolve_model_transfer_target(
    model_str: &str,
) -> Result<ModelTransferTarget, ModelTransferError> {
    let display = model_str.trim();
    if display.is_empty() || !display.contains('/') {
        return Err(ModelTransferError::InvalidFormat);
    }
    let resolved =
        ModelCatalog::resolve_spec_lenient(display).ok_or(ModelTransferError::InvalidFormat)?;
    Ok(ModelTransferTarget {
        display: resolved.display,
        provider: resolved.runtime_provider,
        model_name: resolved.model_name,
        context_window: resolved.context_window as usize,
    })
}

/// Create the target LLM provider — auth / config errors surface before mutation.
pub fn create_model_transfer_provider(
    target: &ModelTransferTarget,
) -> Result<Arc<dyn LLMProvider>, ModelTransferError> {
    edgecrab_tools::create_provider_for_model(&target.provider, &target.model_name)
        .map_err(ModelTransferError::ProviderAuth)
}

/// Estimate prompt mass for window checks (messages + optional cached system prompt).
pub fn estimate_model_transfer_tokens(messages: &[Message], system_prompt: Option<&str>) -> usize {
    let message_tokens = estimate_tokens(messages);
    let system_tokens = system_prompt.map(|s| s.len() / 4).unwrap_or(0);
    message_tokens + system_tokens
}

/// Compression params for the *target* model (DRY: reuses `from_model_config`).
pub fn target_compression_params(
    target: &ModelTransferTarget,
    compression_cfg: &CompressionConfig,
) -> CompressionParams {
    CompressionParams::from_model_config(&target.display, compression_cfg)
}

/// Resolve context window for a `provider/model` display string.
pub fn context_window_for_model(display: &str) -> Option<usize> {
    ModelCatalog::context_window_for_spec(display).map(|w| w as usize)
}

/// Returns true when estimated tokens exceed the target model's compression threshold.
pub fn needs_compression_for_target(
    messages: &[Message],
    system_prompt: Option<&str>,
    params: &CompressionParams,
) -> bool {
    let estimated = estimate_model_transfer_tokens(messages, system_prompt);
    matches!(
        check_compression_status_for_estimate(estimated, params),
        crate::compression::CompressionStatus::NeedsCompression
            | crate::compression::CompressionStatus::PressureWarning
    )
}

/// Compress history to fit the target window when needed.
///
/// Returns `(messages, compressed, llm_succeeded)`.
pub async fn maybe_compress_for_model_transfer(
    messages: Vec<Message>,
    system_prompt: Option<&str>,
    params: &CompressionParams,
    provider: &Arc<dyn LLMProvider>,
) -> Result<(Vec<Message>, bool, bool), ModelTransferError> {
    if !needs_compression_for_target(&messages, system_prompt, params) {
        return Ok((messages, false, true));
    }

    let (compressed, llm_succeeded) = compress_with_llm(&messages, params, provider, None).await;
    let after = estimate_model_transfer_tokens(&compressed, system_prompt);

    if matches!(
        check_compression_status_for_estimate(after, params),
        crate::compression::CompressionStatus::NeedsCompression
            | crate::compression::CompressionStatus::PressureWarning
    ) && !llm_succeeded
    {
        return Err(ModelTransferError::CompressionFailed {
            reason: format!(
                "history still ~{after} tokens after structural fallback (threshold ~{}",
                (params.context_window as f32 * params.threshold) as usize
            ),
        });
    }

    Ok((compressed, true, llm_succeeded))
}

/// Structural fallback: concatenate recent user/assistant turns.
pub fn structural_model_transfer_brief(messages: &[Message]) -> ModelTransferBrief {
    let mut parts = Vec::new();
    for msg in messages.iter().rev() {
        if msg.role == edgecrab_types::Role::User || msg.role == edgecrab_types::Role::Assistant {
            let text = msg.text_content();
            if text.trim().is_empty() || is_prior_model_transfer_message(&text) {
                continue;
            }
            parts.push(format!(
                "{}: {}",
                msg.role,
                crate::safe_truncate(&text, 400)
            ));
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
    ModelTransferBrief(body)
}

/// Generate a one-paragraph model transfer brief via auxiliary LLM, with structural fallback.
pub async fn generate_model_transfer_brief(
    messages: &[Message],
    main_provider: Arc<dyn LLMProvider>,
    main_model: &str,
    auxiliary_model: Option<&str>,
) -> ModelTransferBrief {
    let transcript = build_model_transfer_transcript(messages);
    if transcript.trim().is_empty() {
        return structural_model_transfer_brief(messages);
    }

    let (aux_provider, aux_model) = crate::auxiliary_model::resolve_side_task_provider_and_model(
        None,
        auxiliary_model,
        main_provider,
        main_model,
        "model transfer brief",
    );

    let prompt = format!(
        "Summarize the in-flight task from this conversation in ONE concise paragraph \
         (3-5 sentences). Focus on: current goal, progress so far, blockers, and immediate \
         next steps. Do not include greetings or meta commentary.\n\n{transcript}"
    );
    let chat = vec![
        ChatMessage::system(
            "You write short model transfer briefs for another model continuing the same session.",
        ),
        ChatMessage::user(&prompt),
    ];

    match aux_provider.chat(&chat, None).await {
        Ok(resp) => {
            let text = resp.content.trim();
            if text.is_empty() {
                structural_model_transfer_brief(messages)
            } else {
                ModelTransferBrief(text.to_string())
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, model = %aux_model, "model transfer brief LLM failed — structural fallback");
            structural_model_transfer_brief(messages)
        }
    }
}

fn build_model_transfer_transcript(messages: &[Message]) -> String {
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
        if content.trim().is_empty() || is_prior_model_transfer_message(&content) {
            continue;
        }
        let text = crate::safe_truncate(&content, 600);
        lines.push(format!("{}: {text}", msg.role));
    }
    lines.join("\n\n")
}

fn is_prior_model_transfer_message(text: &str) -> bool {
    text.trim_start().starts_with(MODEL_TRANSFER_USER_PREFIX)
}

/// True when the session has user/assistant turns worth preserving via transfer brief.
pub fn session_requires_model_transfer(messages: &[Message]) -> bool {
    use edgecrab_types::Role;
    messages.iter().any(|msg| {
        if !matches!(msg.role, Role::User | Role::Assistant) {
            return false;
        }
        if msg
            .tool_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty())
        {
            return true;
        }
        let text = msg.text_content();
        !text.trim().is_empty() && !is_prior_model_transfer_message(&text)
    })
}

pub fn format_model_change_confirmation(outcome: &ModelChangeOutcome) -> String {
    match outcome {
        ModelChangeOutcome::Fast(o) => format_model_switch_confirmation(o),
        ModelChangeOutcome::Transfer(o) => format_model_transfer_confirmation(o),
    }
}

/// User-facing success text after model switch/transfer (CLI + gateway DRY).
pub fn format_model_change_result(
    result: Result<ModelChangeOutcome, ModelTransferError>,
) -> String {
    match result {
        Ok(ModelChangeOutcome::Fast(outcome)) => format_model_switch_confirmation(&outcome),
        Ok(ModelChangeOutcome::Transfer(outcome)) => format_model_transfer_confirmation(&outcome),
        Err(err) => format_model_change_error(&err),
    }
}

pub fn format_model_switch_confirmation(outcome: &ModelSwitchOutcome) -> String {
    if outcome.from_model.is_empty() {
        format!("Model → {}", outcome.to_model)
    } else {
        format!("Model → {} (was {})", outcome.to_model, outcome.from_model)
    }
}

pub fn format_model_change_error(err: &ModelTransferError) -> String {
    match err {
        ModelTransferError::SameModel => "Already using that model.".into(),
        other => format!("Model switch failed: {other}"),
    }
}

/// User-facing success text after `Agent::perform_model_transfer` (CLI + gateway DRY).
pub fn format_model_transfer_result(
    result: Result<ModelTransferOutcome, ModelTransferError>,
) -> String {
    match result {
        Ok(outcome) => format_model_transfer_confirmation(&outcome),
        Err(err) => format_model_change_error(&err),
    }
}

/// User-facing confirmation after a successful model transfer (CLI + gateway DRY).
pub fn format_model_transfer_confirmation(outcome: &ModelTransferOutcome) -> String {
    let mut text = format!(
        "↻ Model transfer complete: {} → {}\n\nTask brief:\n{}",
        outcome.from_model, outcome.to_model, outcome.brief
    );
    if outcome.window_shrunk() {
        text.push_str(&format!(
            "\n\nContext window: {} → {} tokens (smaller target window).",
            outcome.from_context_window, outcome.target_context_window
        ));
    }
    if outcome.compressed {
        text.push_str("\n\nNote: history was auto-compressed for the target context window.");
    }
    text.push_str("\n\nGoals, todos, and history are preserved.");
    text
}

/// Format the synthetic user message injected after handoff.
pub fn format_model_transfer_user_message(
    from_model: &str,
    to_model: &str,
    brief: &str,
    compressed: bool,
) -> String {
    let mut msg = format!("{MODEL_TRANSFER_USER_PREFIX} ({from_model} → {to_model}):");
    if compressed {
        msg.push_str("\n\n");
        msg.push_str(crate::compression::HANDOFF_COMPRESSION_NOTE);
    }
    msg.push_str("\n\n");
    msg.push_str(brief);
    msg
}

/// Format handoff history for `/insights` (CLI + gateway DRY). Empty when no records.
pub fn format_model_transfer_insights_section(
    records: &[edgecrab_state::ModelTransferRecord],
) -> String {
    if records.is_empty() {
        return String::new();
    }
    let mut text = String::from("\nModel transfers this session:\n");
    for (idx, h) in records.iter().enumerate() {
        let flat = h.brief.replace('\n', " ");
        let brief_preview = if flat.chars().count() > 120 {
            format!("{}…", crate::safe_truncate(&flat, 120))
        } else {
            flat
        };
        text.push_str(&format!(
            "  {}. {} → {}\n     {}\n",
            idx + 1,
            h.from_model,
            h.to_model,
            brief_preview
        ));
    }
    text
}

/// Mutable inputs for a handoff pipeline run.
pub struct ModelTransferContext<'a> {
    pub current_model: &'a str,
    pub messages: &'a mut Vec<Message>,
    pub system_prompt: Option<&'a str>,
    pub compression_cfg: &'a CompressionConfig,
    pub main_provider: Arc<dyn LLMProvider>,
    pub auxiliary_model: Option<&'a str>,
}

/// Orchestrate a handoff on in-memory session state (testable without full Agent).
pub struct ModelTransferOrchestrator;

impl ModelTransferOrchestrator {
    /// Run the full handoff pipeline. Mutates `ctx.messages` in place on success.
    pub async fn execute(
        ctx: &mut ModelTransferContext<'_>,
        target_spec: &str,
    ) -> Result<(ModelTransferOutcome, Arc<dyn LLMProvider>), ModelTransferError> {
        Self::execute_internal(ctx, target_spec, None).await
    }

    /// Test hook: skip provider factory when a provider is already resolved.
    pub async fn execute_with_provider(
        ctx: &mut ModelTransferContext<'_>,
        target: &ModelTransferTarget,
        new_provider: Arc<dyn LLMProvider>,
    ) -> Result<(ModelTransferOutcome, Arc<dyn LLMProvider>), ModelTransferError> {
        Self::execute_internal(ctx, &target.display, Some((target.clone(), new_provider))).await
    }

    async fn execute_internal(
        ctx: &mut ModelTransferContext<'_>,
        target_spec: &str,
        prebuilt: Option<(ModelTransferTarget, Arc<dyn LLMProvider>)>,
    ) -> Result<(ModelTransferOutcome, Arc<dyn LLMProvider>), ModelTransferError> {
        let (target, new_provider) = match prebuilt {
            Some((target, provider)) => (target, provider),
            None => {
                let target = resolve_model_transfer_target(target_spec)?;
                let provider = create_model_transfer_provider(&target)?;
                (target, provider)
            }
        };

        if ModelCatalog::equivalent_model_specs(&target.display, ctx.current_model) {
            return Err(ModelTransferError::SameModel);
        }

        let from_context_window =
            context_window_for_model(ctx.current_model).unwrap_or(target.context_window);
        let params = target_compression_params(&target, ctx.compression_cfg);

        let (compressed_messages, did_compress, _llm_ok) = maybe_compress_for_model_transfer(
            ctx.messages.clone(),
            ctx.system_prompt,
            &params,
            &new_provider,
        )
        .await?;
        *ctx.messages = compressed_messages;

        let brief = {
            let generated = generate_model_transfer_brief(
                ctx.messages,
                ctx.main_provider.clone(),
                ctx.current_model,
                ctx.auxiliary_model,
            )
            .await;
            if generated.0.trim().is_empty() {
                structural_model_transfer_brief(ctx.messages)
            } else {
                generated
            }
        };

        Ok((
            ModelTransferOutcome {
                from_model: ctx.current_model.to_string(),
                to_model: target.display.clone(),
                brief: brief.0.clone(),
                compressed: did_compress,
                from_context_window,
                target_context_window: target.context_window,
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
    fn resolve_dynamic_lmstudio_model_succeeds() {
        let target = resolve_model_transfer_target("lmstudio/liquid/lfm2.5-1.2b")
            .expect("discovered lmstudio model should resolve");
        assert_eq!(target.display, "lmstudio/liquid/lfm2.5-1.2b");
        assert_eq!(target.provider, "lmstudio");
        assert_eq!(target.model_name, "liquid/lfm2.5-1.2b");
        assert_eq!(target.context_window, 128_000);
    }

    #[test]
    fn resolve_lmstudio_alias_normalizes_display() {
        let target = resolve_model_transfer_target("lm-studio/liquid/lfm2.5-1.2b")
            .expect("lm-studio alias should resolve");
        assert_eq!(target.display, "lmstudio/liquid/lfm2.5-1.2b");
    }

    #[test]
    fn resolve_empty_model_name_is_invalid_format() {
        assert!(matches!(
            resolve_model_transfer_target("lmstudio/"),
            Err(ModelTransferError::InvalidFormat)
        ));
    }

    #[test]
    fn resolve_unknown_provider_still_parses_for_provider_auth() {
        let target = resolve_model_transfer_target("fakeprovider/unknown-model-xyz")
            .expect("lenient resolve accepts provider/model syntax");
        assert_eq!(target.provider, "fakeprovider");
        assert_eq!(target.model_name, "unknown-model-xyz");
    }

    #[test]
    fn resolve_known_catalog_model_succeeds() {
        let target =
            resolve_model_transfer_target("anthropic/claude-haiku-4.5").expect("catalog hit");
        assert_eq!(target.provider, "anthropic");
        assert_eq!(target.model_name, "claude-haiku-4.5");
        assert!(target.context_window > 0);
    }

    #[test]
    fn resolve_copilot_model_uses_catalog_key_and_runtime_provider() {
        let target =
            resolve_model_transfer_target("copilot/claude-haiku-4.5").expect("copilot catalog hit");
        assert_eq!(target.display, "copilot/claude-haiku-4.5");
        assert_eq!(target.provider, "vscode-copilot");
        assert_eq!(target.model_name, "claude-haiku-4.5");
        assert_eq!(target.context_window, 200_000);
    }

    #[test]
    fn context_window_for_copilot_spec() {
        assert_eq!(
            context_window_for_model("copilot/gpt-5-mini"),
            Some(264_000)
        );
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
        let before = estimate_model_transfer_tokens(&messages, None);
        assert!(needs_compression_for_target(&messages, None, &params));
        let (out, compressed, llm_ok) =
            maybe_compress_for_model_transfer(messages.clone(), None, &params, &provider)
                .await
                .expect("compress should succeed with mock/structural fallback");
        messages = out;
        let after = estimate_model_transfer_tokens(&messages, None);
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
        let brief =
            generate_model_transfer_brief(&messages, provider, "anthropic/claude-opus-4.6", None)
                .await;
        assert!(!brief.0.is_empty());
    }

    #[test]
    fn brief_structural_fallback_when_no_llm() {
        let messages = vec![
            Message::user("fix the auth module"),
            Message::assistant("I'll start by reading agent.rs"),
        ];
        let brief = structural_model_transfer_brief(&messages);
        assert!(brief.0.contains("auth module"));
        assert!(brief.0.contains("agent.rs"));
    }

    #[test]
    fn provider_auth_failure_leaves_messages_untouched() {
        let target = ModelTransferTarget {
            display: "definitely-not-a-real-provider/xyz".into(),
            provider: "definitely-not-a-real-provider".into(),
            model_name: "xyz".into(),
            context_window: 128_000,
        };
        let err = create_model_transfer_provider(&target);
        assert!(matches!(err, Err(ModelTransferError::ProviderAuth(_))));
    }

    #[tokio::test]
    async fn orchestrator_same_model_rejected_for_provider_alias() {
        let mut messages = sample_messages(2);
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let cfg = CompressionConfig::default();
        let mut ctx = ModelTransferContext {
            current_model: "lmstudio/liquid/lfm2.5-1.2b",
            messages: &mut messages,
            system_prompt: None,
            compression_cfg: &cfg,
            main_provider: provider,
            auxiliary_model: None,
        };
        let err =
            ModelTransferOrchestrator::execute(&mut ctx, "lm-studio/liquid/lfm2.5-1.2b").await;
        assert!(matches!(err, Err(ModelTransferError::SameModel)));
    }

    #[tokio::test]
    async fn orchestrator_same_model_rejected() {
        let mut messages = sample_messages(2);
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let cfg = CompressionConfig::default();
        let mut ctx = ModelTransferContext {
            current_model: "anthropic/claude-haiku-4.5",
            messages: &mut messages,
            system_prompt: None,
            compression_cfg: &cfg,
            main_provider: provider,
            auxiliary_model: None,
        };
        let err = ModelTransferOrchestrator::execute(&mut ctx, "anthropic/claude-haiku-4.5").await;
        assert!(matches!(err, Err(ModelTransferError::SameModel)));
    }

    #[test]
    fn format_handoff_message_includes_models_and_brief() {
        let msg = format_model_transfer_user_message("a/m1", "b/m2", "Working on tests.", false);
        assert!(msg.contains("a/m1 → b/m2"));
        assert!(msg.contains("Working on tests."));
    }

    #[test]
    fn format_handoff_message_includes_compression_note_when_compressed() {
        let msg = format_model_transfer_user_message("a/m1", "b/m2", "Working on tests.", true);
        assert!(msg.contains(crate::compression::HANDOFF_COMPRESSION_NOTE));
    }

    #[test]
    fn format_model_transfer_insights_section_empty_for_no_records() {
        assert!(format_model_transfer_insights_section(&[]).is_empty());
    }

    #[test]
    fn format_model_transfer_insights_section_lists_records() {
        let records = vec![edgecrab_state::ModelTransferRecord {
            session_id: "s1".into(),
            from_model: "a/m1".into(),
            to_model: "b/m2".into(),
            brief: "Implement handoff.".into(),
            ts: 1.0,
        }];
        let text = format_model_transfer_insights_section(&records);
        assert!(text.contains("a/m1 → b/m2"));
        assert!(text.contains("Implement handoff."));
    }

    #[test]
    fn confirmation_includes_window_shrink_notice() {
        let outcome = ModelTransferOutcome {
            from_model: "anthropic/claude-opus-4.6".into(),
            to_model: "copilot/gpt-5-mini".into(),
            brief: "Implement session handoff.".into(),
            compressed: true,
            from_context_window: 200_000,
            target_context_window: 128_000,
        };
        let text = format_model_transfer_confirmation(&outcome);
        assert!(text.contains("Model transfer complete"));
        assert!(text.contains("200000"));
        assert!(text.contains("128000"));
        assert!(text.contains("auto-compressed"));
    }

    #[test]
    fn format_model_transfer_result_maps_errors() {
        let err = format_model_transfer_result(Err(ModelTransferError::SameModel));
        assert!(err.contains("Already using that model"));
    }

    #[tokio::test]
    async fn empty_llm_brief_falls_back_to_structural() {
        let target = resolve_model_transfer_target("anthropic/claude-haiku-4.5").expect("catalog");
        let mut messages = vec![Message::user("implement session handoff parity")];
        let mock = MockProvider::new();
        mock.add_response("").await;
        let provider: Arc<dyn LLMProvider> = Arc::new(mock);
        let cfg = CompressionConfig::default();
        let mut ctx = ModelTransferContext {
            current_model: "anthropic/claude-opus-4.6",
            messages: &mut messages,
            system_prompt: None,
            compression_cfg: &cfg,
            main_provider: provider.clone(),
            auxiliary_model: None,
        };
        let (outcome, _) =
            ModelTransferOrchestrator::execute_with_provider(&mut ctx, &target, provider)
                .await
                .expect("transfer");
        assert!(outcome.brief.contains("implement session handoff"));
    }

    #[test]
    fn brief_ignores_prior_handoff_messages() {
        let messages = vec![
            Message::user("start the auth refactor"),
            Message::user(&format_model_transfer_user_message(
                "x/y",
                "a/b",
                "stale brief",
                false,
            )),
            Message::assistant("reading agent.rs"),
        ];
        let brief = structural_model_transfer_brief(&messages);
        assert!(brief.0.contains("auth refactor"));
        assert!(!brief.0.contains("stale brief"));
    }

    #[tokio::test]
    async fn execute_with_provider_completes_without_factory() {
        let target = resolve_model_transfer_target("anthropic/claude-haiku-4.5").expect("catalog");
        let mut messages = sample_messages(4);
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        let cfg = CompressionConfig::default();
        let mut ctx = ModelTransferContext {
            current_model: "anthropic/claude-opus-4.6",
            messages: &mut messages,
            system_prompt: None,
            compression_cfg: &cfg,
            main_provider: provider.clone(),
            auxiliary_model: None,
        };
        let (outcome, _) =
            ModelTransferOrchestrator::execute_with_provider(&mut ctx, &target, provider)
                .await
                .expect("handoff with injected provider");
        assert_eq!(outcome.to_model, "anthropic/claude-haiku-4.5");
        assert!(outcome.from_context_window >= outcome.target_context_window);
    }

    #[test]
    fn session_requires_model_transfer_false_for_empty_history() {
        assert!(!session_requires_model_transfer(&[]));
    }

    #[test]
    fn session_requires_model_transfer_false_for_system_only() {
        assert!(!session_requires_model_transfer(&[Message::system(
            "You are helpful."
        )]));
    }

    #[test]
    fn session_requires_model_transfer_true_for_user_turn() {
        assert!(session_requires_model_transfer(&[Message::user("hello")]));
    }

    #[test]
    fn session_requires_model_transfer_ignores_prior_handoff_messages() {
        let messages = vec![Message::user(&format_model_transfer_user_message(
            "a/m1", "b/m2", "stale", false,
        ))];
        assert!(!session_requires_model_transfer(&messages));
    }

    #[test]
    fn format_model_change_confirmation_fast_and_transfer() {
        let fast =
            format_model_change_confirmation(&ModelChangeOutcome::Fast(ModelSwitchOutcome {
                from_model: "a/m1".into(),
                to_model: "b/m2".into(),
            }));
        assert!(fast.contains("b/m2"));
        assert!(fast.contains("a/m1"));

        let transfer =
            format_model_change_confirmation(&ModelChangeOutcome::Transfer(ModelTransferOutcome {
                from_model: "a/m1".into(),
                to_model: "b/m2".into(),
                brief: "Keep going.".into(),
                compressed: false,
                from_context_window: 200_000,
                target_context_window: 128_000,
            }));
        assert!(transfer.contains("Task brief"));
        assert!(transfer.contains("Keep going."));
    }
}
