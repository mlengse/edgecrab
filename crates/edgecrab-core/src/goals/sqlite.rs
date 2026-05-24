//! SQLite-backed persistent goal store (per session_id).

use std::sync::Arc;

use edgecrab_state::{SessionDb, StoredGoalState, StoredSubGoal};
use edgecrab_types::AgentError;

use super::{GoalState, GoalStore, SubGoal};

fn map_state(stored: StoredGoalState) -> GoalState {
    GoalState {
        goal_text: stored.goal_text,
        subgoals: stored
            .subgoals
            .into_iter()
            .map(|s| SubGoal {
                id: s.id,
                text: s.text,
                done: s.done,
            })
            .collect(),
    }
}

fn map_subgoal(sub: StoredSubGoal) -> SubGoal {
    SubGoal {
        id: sub.id,
        text: sub.text,
        done: sub.done,
    }
}

pub struct SqliteGoalStore {
    db: Arc<SessionDb>,
}

impl SqliteGoalStore {
    pub fn new(db: Arc<SessionDb>) -> Self {
        Self { db }
    }
}

impl GoalStore for SqliteGoalStore {
    fn active(&self, session_id: &str) -> Result<GoalState, AgentError> {
        Ok(map_state(self.db.goals_active(session_id)?))
    }

    fn set_goal(&self, session_id: &str, text: &str) -> Result<(), AgentError> {
        self.db.goals_set(session_id, text)
    }

    fn clear(&self, session_id: &str) -> Result<(), AgentError> {
        self.db.goals_clear(session_id)
    }

    fn push_subgoal(&self, session_id: &str, text: &str) -> Result<(), AgentError> {
        match self.db.goals_push_subgoal(session_id, text) {
            Ok(()) => Ok(()),
            Err(AgentError::Database(msg)) if msg.contains("top-level goal") => {
                Err(AgentError::Config(msg))
            }
            Err(err) => Err(err),
        }
    }

    fn complete_subgoal(&self, session_id: &str) -> Result<Option<SubGoal>, AgentError> {
        Ok(self
            .db
            .goals_complete_subgoal(session_id)?
            .map(map_subgoal))
    }
}
