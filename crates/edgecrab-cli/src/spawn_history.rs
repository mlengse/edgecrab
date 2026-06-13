//! Completed delegate snapshots for `/agents` history (Hermes `spawnHistoryStore` MVP).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::turn_activity::ShelfSubagentRow;

pub const SPAWN_HISTORY_MAX: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurnCommitMetrics {
    pub token_est: u32,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpawnHistoryEntry {
    pub task_index: usize,
    pub task_count: usize,
    pub goal: String,
    pub agent_id: String,
    pub parent_id: Option<String>,
    pub depth: u32,
    pub tool_count: usize,
    pub duration_secs: u64,
    pub status: String,
}

/// One fan-out turn — aggregates all delegates that finished in the same agent turn.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnTurnSnapshot {
    pub label: String,
    pub delegates: Vec<SpawnHistoryEntry>,
    pub total_tools: usize,
    pub total_duration_secs: u64,
    /// Max parallel fan-out width in the turn (Hermes tree breadth proxy).
    pub max_fanout: usize,
    /// Shelf token estimate for the turn (thinking + tool rough counts).
    pub token_est: u32,
    /// Turn cost delta from session snapshot (USD).
    pub cost_usd: f64,
}

impl SpawnTurnSnapshot {
    pub fn from_entries(
        label: String,
        delegates: Vec<SpawnHistoryEntry>,
        metrics: TurnCommitMetrics,
    ) -> Self {
        let total_tools = delegates.iter().map(|d| d.tool_count).sum();
        let total_duration_secs = delegates
            .iter()
            .map(|d| d.duration_secs)
            .max()
            .unwrap_or(0);
        let max_fanout = delegates
            .iter()
            .map(|d| d.task_count)
            .max()
            .unwrap_or(0);
        Self {
            label,
            delegates,
            total_tools,
            total_duration_secs,
            max_fanout,
            token_est: metrics.token_est,
            cost_usd: metrics.cost_usd,
        }
    }

    pub fn delegate_count(&self) -> usize {
        self.delegates.len()
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpawnHistory {
    turns: VecDeque<SpawnTurnSnapshot>,
    pending_turn: Vec<SpawnHistoryEntry>,
}

impl SpawnHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_finish(
        &mut self,
        row: &ShelfSubagentRow,
        duration_secs: u64,
        status: &str,
    ) {
        self.pending_turn.push(SpawnHistoryEntry {
            task_index: row.task_index,
            task_count: row.task_count,
            goal: row.goal.clone(),
            agent_id: row.agent_id.clone(),
            parent_id: row.parent_id.clone(),
            depth: row.depth,
            tool_count: row.tool_count,
            duration_secs,
            status: status.to_string(),
        });
    }

    /// Commit pending delegate finishes as one turn snapshot (call on turn `Done`).
    pub fn commit_turn(&mut self, metrics: TurnCommitMetrics) {
        if self.pending_turn.is_empty() {
            return;
        }
        let label = summarize_turn_label(&self.pending_turn);
        let snapshot =
            SpawnTurnSnapshot::from_entries(label, std::mem::take(&mut self.pending_turn), metrics);
        self.turns.push_front(snapshot);
        while self.turns.len() > SPAWN_HISTORY_MAX {
            self.turns.pop_back();
        }
    }

    pub fn turns(&self) -> impl DoubleEndedIterator<Item = &SpawnTurnSnapshot> {
        self.turns.iter()
    }

    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    pub fn clear(&mut self) {
        self.turns.clear();
        self.pending_turn.clear();
    }

    /// Push a disk-loaded snapshot at the front (Hermes `pushDiskSnapshot`).
    pub fn push_disk_snapshot(&mut self, snapshot: SpawnTurnSnapshot) {
        if snapshot.delegates.is_empty() {
            return;
        }
        self.turns.push_front(snapshot);
        while self.turns.len() > SPAWN_HISTORY_MAX {
            self.turns.pop_back();
        }
    }
}

fn summarize_turn_label(entries: &[SpawnHistoryEntry]) -> String {
    let goals: Vec<&str> = entries
        .iter()
        .take(2)
        .map(|e| e.goal.as_str())
        .filter(|g| !g.is_empty())
        .collect();
    if goals.is_empty() {
        format!("{} delegate(s)", entries.len())
    } else if entries.len() > 2 {
        format!("{} · +{} more", goals.join(" · "), entries.len() - 2)
    } else {
        goals.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn sample_row(i: usize, tools: usize) -> ShelfSubagentRow {
        ShelfSubagentRow {
            task_index: i,
            task_count: 2,
            depth: 1,
            goal: format!("goal-{i}"),
            agent_id: format!("sa-{i}"),
            parent_id: None,
            detail: None,
            tool_count: tools,
            current_tool: None,
            started_at: Instant::now(),
            recent_tools: Vec::new(),
        }
    }

    #[test]
    fn caps_turn_history_length() {
        let mut history = SpawnHistory::new();
        for i in 0..12 {
            history.record_finish(&sample_row(i, i), i as u64, "completed");
            history.commit_turn(TurnCommitMetrics::default());
        }
        assert_eq!(history.turn_count(), SPAWN_HISTORY_MAX);
    }

    #[test]
    fn commit_groups_delegates_into_one_turn() {
        let mut history = SpawnHistory::new();
        history.record_finish(&sample_row(0, 3), 10, "completed");
        history.record_finish(&sample_row(1, 5), 20, "completed");
        history.commit_turn(TurnCommitMetrics::default());
        assert_eq!(history.turn_count(), 1);
        let turn = history.turns().next().unwrap();
        assert_eq!(turn.delegate_count(), 2);
        assert_eq!(turn.total_tools, 8);
        assert_eq!(turn.total_duration_secs, 20);
    }

    #[test]
    fn empty_commit_is_noop() {
        let mut history = SpawnHistory::new();
        history.commit_turn(TurnCommitMetrics::default());
        assert_eq!(history.turn_count(), 0);
    }
}
