//! Goal judge — auxiliary LLM call that decides whether a standing goal is done.
//!
//! Mirrors hermes-agent `hermes_cli/goals.py::judge_goal`. Fail-open on errors.

use std::sync::Arc;

use edgequake_llm::LLMProvider;

use crate::config::GoalJudgeConfig;
use crate::goals::GoalState;

pub const DEFAULT_JUDGE_MAX_TOKENS: u32 = 4096;
pub const DEFAULT_MAX_CONSECUTIVE_PARSE_FAILURES: u32 = 3;
const JUDGE_RESPONSE_SNIPPET_CHARS: usize = 4000;

const JUDGE_SYSTEM_PROMPT: &str = "\
You are a strict judge evaluating whether an autonomous agent has \
achieved a user's stated goal. You receive the goal text and the \
agent's most recent response. Your only job is to decide whether \
the goal is fully satisfied based on that response.\n\n\
A goal is DONE only when:\n\
- The response explicitly confirms the goal was completed, OR\n\
- The response clearly shows the final deliverable was produced, OR\n\
- The response explains the goal is unachievable / blocked / needs \
user input (treat this as DONE with reason describing the block).\n\n\
Otherwise the goal is NOT done — CONTINUE.\n\n\
Reply ONLY with a single JSON object on one line:\n\
{\"done\": <true|false>, \"reason\": \"<one-sentence rationale>\"}";

const JUDGE_USER_PROMPT: &str = "\
Goal:\n{goal}\n\n\
Agent's most recent response:\n{response}\n\n\
Current time: {current_time}\n\n\
Is the goal satisfied?";

const JUDGE_USER_PROMPT_WITH_SUBGOALS: &str = "\
Goal:\n{goal}\n\n\
Additional criteria the user added mid-loop (all must also be \
satisfied for the goal to be DONE):\n{subgoals_block}\n\n\
Agent's most recent response:\n{response}\n\n\
Current time: {current_time}\n\n\
Decision: For each numbered criterion above, find concrete \
evidence in the agent's response that the criterion is \
satisfied. Do not accept generic phrases like 'all requirements \
met' or 'implying it was done' — require specific evidence (a \
file contents excerpt, an output line, a command result). If \
ANY criterion lacks specific evidence in the response, the goal \
is NOT done — return CONTINUE.\n\n\
Is the goal AND every additional criterion satisfied?";

/// Parsed judge verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalJudgeVerdict {
    pub done: bool,
    pub reason: String,
    pub parse_failed: bool,
}

/// Run the goal judge against the last assistant response.
pub async fn run_goal_judge(
    provider: &Arc<dyn LLMProvider>,
    _model: &str,
    goal: &str,
    last_response: &str,
    state: &GoalState,
    judge_cfg: &GoalJudgeConfig,
) -> GoalJudgeVerdict {
    if goal.trim().is_empty() {
        return GoalJudgeVerdict {
            done: false,
            reason: "empty goal".into(),
            parse_failed: false,
        };
    }
    if last_response.trim().is_empty() {
        return GoalJudgeVerdict {
            done: false,
            reason: "empty response (nothing to evaluate)".into(),
            parse_failed: false,
        };
    }

    let now = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S %Z")
        .to_string();
    let active_subgoals: Vec<String> = state
        .subgoals
        .iter()
        .map(|s| s.text.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let user_prompt = if active_subgoals.is_empty() {
        JUDGE_USER_PROMPT
            .replace("{goal}", &truncate(goal, 2000))
            .replace(
                "{response}",
                &truncate(last_response, JUDGE_RESPONSE_SNIPPET_CHARS),
            )
            .replace("{current_time}", &now)
    } else {
        let subgoals_block = active_subgoals
            .iter()
            .enumerate()
            .map(|(i, text)| format!("- {}. {text}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        JUDGE_USER_PROMPT_WITH_SUBGOALS
            .replace("{goal}", &truncate(goal, 2000))
            .replace("{subgoals_block}", &truncate(&subgoals_block, 2000))
            .replace(
                "{response}",
                &truncate(last_response, JUDGE_RESPONSE_SNIPPET_CHARS),
            )
            .replace("{current_time}", &now)
    };

    let max_tokens = judge_cfg.max_tokens.max(1) as usize;
    let messages = vec![
        edgequake_llm::ChatMessage::system(JUDGE_SYSTEM_PROMPT),
        edgequake_llm::ChatMessage::user(&user_prompt),
    ];
    let options = edgequake_llm::CompletionOptions {
        max_tokens: Some(max_tokens),
        temperature: Some(0.0),
        ..Default::default()
    };

    let response = match provider
        .chat_with_tools(&messages, &[], None, Some(&options))
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            tracing::info!(
                error = %err,
                "goal judge: API call failed — falling through to continue"
            );
            return GoalJudgeVerdict {
                done: false,
                reason: format!("judge error: {err}"),
                parse_failed: false,
            };
        }
    };

    let raw = response.content.trim().to_string();
    parse_judge_response(&raw)
}

/// Resolve `(provider, model)` for the goal judge (mirrors shadow_judge resolution).
pub fn resolve_goal_judge_provider_and_model(
    judge_cfg: &GoalJudgeConfig,
    auxiliary_model: Option<&str>,
    main_provider: Arc<dyn LLMProvider>,
    main_model: &str,
) -> (Arc<dyn LLMProvider>, String) {
    crate::auxiliary_model::resolve_side_task_provider_and_model(
        judge_cfg.model.as_deref(),
        auxiliary_model,
        main_provider,
        main_model,
        "goal judge",
    )
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let end = text
        .char_indices()
        .nth(limit)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());
    format!("{}… [truncated]", &text[..end])
}

/// Parse judge JSON. Fail-open to `(done=false, reason, parse_failed=true)`.
pub fn parse_judge_response(raw: &str) -> GoalJudgeVerdict {
    if raw.trim().is_empty() {
        return GoalJudgeVerdict {
            done: false,
            reason: "judge returned empty response".into(),
            parse_failed: true,
        };
    }

    let mut text = raw.trim().to_string();
    if text.starts_with("```") {
        text = text.trim_matches('`').to_string();
        if let Some(nl) = text.find('\n') {
            text = text[nl + 1..].to_string();
        }
    }

    let data = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .or_else(|| extract_json_object(&text));

    let Some(data) = data.filter(|v| v.is_object()) else {
        return GoalJudgeVerdict {
            done: false,
            reason: format!("judge reply was not JSON: {:?}", truncate(raw, 200)),
            parse_failed: true,
        };
    };

    let done_val = &data["done"];
    let done = if let Some(s) = done_val.as_str() {
        matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "true" | "yes" | "1" | "done"
        )
    } else {
        done_val.as_bool().unwrap_or(false)
    };
    let reason = data["reason"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("no reason provided")
        .to_string();

    GoalJudgeVerdict {
        done,
        reason,
        parse_failed: false,
    }
}

fn extract_json_object(text: &str) -> Option<serde_json::Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')? + 1;
    serde_json::from_str(&text[start..end]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_done_json() {
        let v = parse_judge_response(r#"{"done": true, "reason": "Shipped feature"}"#);
        assert!(v.done);
        assert_eq!(v.reason, "Shipped feature");
        assert!(!v.parse_failed);
    }

    #[test]
    fn parse_continue_json() {
        let v = parse_judge_response(r#"{"done": false, "reason": "Still working"}"#);
        assert!(!v.done);
        assert!(!v.parse_failed);
    }

    #[test]
    fn parse_empty_is_parse_failure() {
        let v = parse_judge_response("");
        assert!(!v.done);
        assert!(v.parse_failed);
    }

    #[test]
    fn parse_strips_markdown_fence() {
        let v = parse_judge_response("```json\n{\"done\": true, \"reason\": \"ok\"}\n```");
        assert!(v.done);
        assert!(!v.parse_failed);
    }

    #[test]
    fn parse_prose_wrapped_json() {
        let v = parse_judge_response(
            "Here is my verdict: {\"done\": false, \"reason\": \"needs tests\"}",
        );
        assert!(!v.done);
        assert_eq!(v.reason, "needs tests");
    }
}
