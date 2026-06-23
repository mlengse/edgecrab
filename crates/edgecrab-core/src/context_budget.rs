//! Startup / turn-1 context token budget estimation (CI + `/context budget`).
//!
//! Uses the same chars÷4 heuristic as `conversation.rs` so numbers match
//! compression gates and shelf displays.

use edgecrab_types::Message;
use edgequake_llm::ToolDefinition;

/// Rough token estimate from character count (mixed code + English).
pub fn estimate_chars_as_tokens(text: &str) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0;
    }
    trimmed.chars().count().div_ceil(4)
}

pub fn estimate_json_tokens<T: serde::Serialize + ?Sized>(value: &T) -> usize {
    serde_json::to_string(value)
        .map(|s| estimate_chars_as_tokens(&s))
        .unwrap_or(0)
}

/// Token breakdown for doctor / slash-command display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBudgetBreakdown {
    pub stable_tokens: usize,
    pub dynamic_tokens: usize,
    pub tools_tokens: usize,
    pub tool_count: usize,
    pub history_tokens: usize,
    pub total_tokens: usize,
    pub context_window: usize,
}

impl ContextBudgetBreakdown {
    pub fn pct_of_window(&self) -> f64 {
        if self.context_window == 0 {
            return 0.0;
        }
        (self.total_tokens as f64 / self.context_window as f64) * 100.0
    }

    pub fn format_report(&self) -> String {
        format!(
            "Context budget (estimated):\n\
             \n\
             stable:    {:>6} tok\n\
             dynamic:   {:>6} tok\n\
             tools:     {:>6} tok ({} tools)\n\
             history:   {:>6} tok\n\
             ─────────────────────\n\
             total:     {:>6} tok ({:.1}% of {}K)",
            self.stable_tokens,
            self.dynamic_tokens,
            self.tools_tokens,
            self.tool_count,
            self.history_tokens,
            self.total_tokens,
            self.pct_of_window(),
            self.context_window / 1000,
        )
    }
}

/// Estimate full request mass: system zones + tool schemas + conversation history.
pub fn estimate_context_budget(
    stable_prompt: Option<&str>,
    dynamic_prompt: Option<&str>,
    combined_system: Option<&str>,
    messages: &[Message],
    tool_defs: &[ToolDefinition],
    context_window: usize,
) -> ContextBudgetBreakdown {
    let (stable_tokens, dynamic_tokens) = match (stable_prompt, dynamic_prompt) {
        (Some(stable), Some(dynamic)) => (
            estimate_chars_as_tokens(stable),
            estimate_chars_as_tokens(dynamic),
        ),
        _ => {
            let combined = combined_system.unwrap_or("");
            (0, estimate_chars_as_tokens(combined))
        }
    };

    let tools_tokens = estimate_json_tokens(tool_defs);
    let tool_count = tool_defs.len();
    let history_tokens = crate::compression::estimate_tokens(messages);
    let total_tokens = stable_tokens + dynamic_tokens + tools_tokens + history_tokens;

    ContextBudgetBreakdown {
        stable_tokens,
        dynamic_tokens,
        tools_tokens,
        tool_count,
        history_tokens,
        total_tokens,
        context_window,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgecrab_tools::{AppConfigRef, ToolContext, ToolRegistry};
    use edgecrab_types::Platform;

    fn schema_tokens_for_toolsets(alias: &str) -> usize {
        let registry = ToolRegistry::new();
        let ctx = ToolContext {
            task_id: "budget-test".into(),
            cwd: std::env::temp_dir(),
            session_id: "budget-test".into(),
            user_task: None,
            cancel: tokio_util::sync::CancellationToken::new(),
            config: AppConfigRef::default(),
            state_db: None,
            platform: Platform::Cli,
            process_table: None,
            provider: None,
            tool_registry: None,
            delegate_depth: 0,
            delegate_agent_id: None,
            delegate_parent_id: None,
            sub_agent_runner: None,
            delegation_event_tx: None,
            clarify_tx: None,
            approval_tx: None,
            on_skills_changed: None,
            gateway_sender: None,
            origin_chat: None,
            session_key: None,
            todo_store: None,
            current_tool_call_id: None,
            current_tool_name: None,
            injected_messages: None,
            tool_progress_tx: None,
            watch_notification_tx: None,
            mutation_turn: None,
            lsp_gate: None,
            kanban_task_id: None,
        };
        let enabled =
            edgecrab_tools::toolsets::expand_toolset_names(&[alias.to_string()]);
        let schemas = registry.get_definitions(Some(&enabled), None, &ctx);
        let llm = edgecrab_tools::to_llm_definitions(&schemas);
        estimate_json_tokens(&llm)
    }

    #[test]
    fn default_core_profile_under_18k_schema_tokens() {
        let tokens = schema_tokens_for_toolsets("core");
        assert!(
            tokens < 18_000,
            "core toolset schema budget exceeded: {tokens} tok (limit 18_000)"
        );
    }

    #[test]
    fn minimal_profile_under_8k_schema_tokens() {
        let tokens = schema_tokens_for_toolsets("minimal");
        assert!(
            tokens < 8_000,
            "minimal toolset schema budget exceeded: {tokens} tok (limit 8_000)"
        );
    }

    #[test]
    fn format_report_includes_sections() {
        let breakdown = ContextBudgetBreakdown {
            stable_tokens: 2847,
            dynamic_tokens: 1203,
            tools_tokens: 14992,
            tool_count: 35,
            history_tokens: 500,
            total_tokens: 19542,
            context_window: 128_000,
        };
        let text = breakdown.format_report();
        assert!(text.contains("stable:"));
        assert!(text.contains("35 tools"));
        assert!(text.contains("128K"));
    }
}
