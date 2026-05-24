//! Persistent session goals — Ralph loop intent re-injection.
//!
//! Goals live outside the conversation message list so they survive
//! `/compress`. Each ReAct iteration appends a synthetic **user** message
//! (never a system-prompt mutation) to preserve Anthropic prompt cache.

mod sqlite;

use std::sync::{Arc, Mutex};

use edgecrab_types::AgentError;

pub use sqlite::SqliteGoalStore;

/// One sub-step under the active top-level goal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubGoal {
    pub id: u64,
    pub text: String,
    pub done: bool,
}

/// Active goal state for a session.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GoalState {
    pub goal_text: Option<String>,
    pub subgoals: Vec<SubGoal>,
}

impl GoalState {
    pub fn is_empty(&self) -> bool {
        self.goal_text.as_ref().is_none_or(|t| t.trim().is_empty())
    }
}

/// Storage backend for persistent goals (≤ 5 methods — ISP guard).
pub trait GoalStore: Send + Sync {
    fn active(&self, session_id: &str) -> Result<GoalState, AgentError>;
    fn set_goal(&self, session_id: &str, text: &str) -> Result<(), AgentError>;
    fn clear(&self, session_id: &str) -> Result<(), AgentError>;
    fn push_subgoal(&self, session_id: &str, text: &str) -> Result<(), AgentError>;
    fn complete_subgoal(&self, session_id: &str) -> Result<Option<SubGoal>, AgentError>;
}

/// Render the per-turn injection block shown to the model.
pub fn render_goal_block(state: &GoalState) -> String {
    let Some(goal) = state
        .goal_text
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    else {
        return String::new();
    };

    let mut lines = vec![
        "[GOAL CONTEXT — auto-injected each turn]".to_string(),
        format!("Active goal: {goal}"),
    ];

    if !state.subgoals.is_empty() {
        lines.push("Subgoals:".to_string());
        for (idx, sub) in state.subgoals.iter().enumerate() {
            let marker = if sub.done { "[x]" } else { "[ ]" };
            lines.push(format!("  {}. {marker} {}", idx + 1, sub.text));
        }
    }

    lines.join("\n")
}

/// In-memory fallback when no SQLite state DB is configured (tests / minimal runs).
pub struct InMemoryGoalStore {
    sessions: Mutex<std::collections::HashMap<String, GoalState>>,
}

impl Default for InMemoryGoalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryGoalStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn with_session<F, T>(&self, session_id: &str, f: F) -> Result<T, AgentError>
    where
        F: FnOnce(&mut GoalState) -> Result<T, AgentError>,
    {
        if session_id.trim().is_empty() {
            return Err(AgentError::Config(
                "session_id is required for goal operations".into(),
            ));
        }
        let mut guard = self
            .sessions
            .lock()
            .map_err(|_| AgentError::Database("goal store mutex poisoned".into()))?;
        f(guard.entry(session_id.to_string()).or_default())
    }
}

impl GoalStore for InMemoryGoalStore {
    fn active(&self, session_id: &str) -> Result<GoalState, AgentError> {
        if session_id.trim().is_empty() {
            return Ok(GoalState::default());
        }
        let guard = self
            .sessions
            .lock()
            .map_err(|_| AgentError::Database("goal store mutex poisoned".into()))?;
        Ok(guard.get(session_id).cloned().unwrap_or_default())
    }

    fn set_goal(&self, session_id: &str, text: &str) -> Result<(), AgentError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(AgentError::Config("goal text must not be empty".into()));
        }
        self.with_session(session_id, |state| {
            state.goal_text = Some(trimmed.to_string());
            state.subgoals.clear();
            Ok(())
        })
    }

    fn clear(&self, session_id: &str) -> Result<(), AgentError> {
        if session_id.trim().is_empty() {
            return Ok(());
        }
        let mut guard = self
            .sessions
            .lock()
            .map_err(|_| AgentError::Database("goal store mutex poisoned".into()))?;
        guard.remove(session_id);
        Ok(())
    }

    fn push_subgoal(&self, session_id: &str, text: &str) -> Result<(), AgentError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(AgentError::Config("subgoal text must not be empty".into()));
        }
        self.with_session(session_id, |state| {
            if state.goal_text.as_ref().is_none_or(|g| g.trim().is_empty()) {
                return Err(AgentError::Config(
                    "set a top-level goal with /goal before adding subgoals".into(),
                ));
            }
            let next_id = state
                .subgoals
                .iter()
                .map(|s| s.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            state.subgoals.push(SubGoal {
                id: next_id,
                text: trimmed.to_string(),
                done: false,
            });
            Ok::<(), AgentError>(())
        })
    }

    fn complete_subgoal(&self, session_id: &str) -> Result<Option<SubGoal>, AgentError> {
        self.with_session(session_id, |state| {
            if let Some(sub) = state.subgoals.iter_mut().rev().find(|s| !s.done) {
                sub.done = true;
                return Ok(Some(sub.clone()));
            }
            Ok(None)
        })
    }
}

/// Build the default goal store for an agent (SQLite when a state DB exists).
pub fn goal_store_for_db(db: Option<Arc<edgecrab_state::SessionDb>>) -> Arc<dyn GoalStore> {
    match db {
        Some(db) => Arc::new(SqliteGoalStore::new(db)),
        None => Arc::new(InMemoryGoalStore::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> InMemoryGoalStore {
        InMemoryGoalStore::new()
    }

    #[test]
    fn empty_store_renders_nothing() {
        let state = GoalState::default();
        assert!(render_goal_block(&state).is_empty());
        assert!(store().active("s1").expect("active").is_empty());
    }

    #[test]
    fn set_goal_and_show() {
        let s = store();
        s.set_goal("sess", "Refactor payment service").expect("set");
        let state = s.active("sess").expect("active");
        assert_eq!(
            state.goal_text.as_deref(),
            Some("Refactor payment service")
        );
        let block = render_goal_block(&state);
        assert!(block.contains("Active goal: Refactor payment service"));
        assert!(block.contains("[GOAL CONTEXT"));
    }

    #[test]
    fn push_subgoal_ordering() {
        let s = store();
        s.set_goal("sess", "Ship feature").expect("set");
        s.push_subgoal("sess", "step 1").expect("push");
        s.push_subgoal("sess", "step 2").expect("push");
        let state = s.active("sess").expect("active");
        assert_eq!(state.subgoals.len(), 2);
        assert_eq!(state.subgoals[0].text, "step 1");
        assert_eq!(state.subgoals[1].text, "step 2");
    }

    #[test]
    fn complete_subgoal_marks_most_recent_undone() {
        let s = store();
        s.set_goal("sess", "Ship feature").expect("set");
        s.push_subgoal("sess", "step 1").expect("push");
        s.push_subgoal("sess", "step 2").expect("push");
        let done = s.complete_subgoal("sess").expect("done");
        assert_eq!(done.as_ref().map(|d| d.text.as_str()), Some("step 2"));
        let state = s.active("sess").expect("active");
        assert!(!state.subgoals[0].done);
        assert!(state.subgoals[1].done);
        let block = render_goal_block(&state);
        assert!(block.contains("[ ] step 1"));
        assert!(block.contains("[x] step 2"));
    }

    #[test]
    fn two_session_isolation() {
        let s = store();
        s.set_goal("a", "Goal A").expect("set a");
        s.set_goal("b", "Goal B").expect("set b");
        assert_eq!(
            s.active("a").expect("a").goal_text.as_deref(),
            Some("Goal A")
        );
        assert_eq!(
            s.active("b").expect("b").goal_text.as_deref(),
            Some("Goal B")
        );
    }

    #[test]
    fn clear_empties_session_only() {
        let s = store();
        s.set_goal("a", "Goal A").expect("set a");
        s.set_goal("b", "Goal B").expect("set b");
        s.clear("a").expect("clear");
        assert!(s.active("a").expect("a").is_empty());
        assert_eq!(
            s.active("b").expect("b").goal_text.as_deref(),
            Some("Goal B")
        );
    }

    #[test]
    fn json_round_trip() {
        let state = GoalState {
            goal_text: Some("Test".into()),
            subgoals: vec![SubGoal {
                id: 1,
                text: "sub".into(),
                done: true,
            }],
        };
        let json = serde_json::to_string(&state).expect("serialize");
        let back: GoalState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, back);
    }

    #[test]
    fn set_goal_replaces_subgoals() {
        let s = store();
        s.set_goal("sess", "First").expect("set");
        s.push_subgoal("sess", "sub").expect("push");
        s.set_goal("sess", "Second").expect("replace");
        let state = s.active("sess").expect("active");
        assert_eq!(state.goal_text.as_deref(), Some("Second"));
        assert!(state.subgoals.is_empty());
    }

    #[test]
    fn push_subgoal_requires_top_level_goal() {
        let s = store();
        let err = s.push_subgoal("sess", "orphan").expect_err("needs goal");
        assert!(err.to_string().contains("top-level goal"));
    }
}
