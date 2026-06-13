//! Compact delegation cap HUD for the status bar — Hermes `SpawnHud` parity.

use crate::subagent_tree::width_by_depth;
use crate::turn_activity::TurnActivityState;

/// Matches `delegate_task.rs` — parent(0) → child(1); depth ≥ MAX_DEPTH rejected.
pub const DEFAULT_MAX_SPAWN_DEPTH: u32 = 1;

/// Matches `delegate_task.rs` `MAX_CONCURRENT_CHILDREN`.
pub const DEFAULT_MAX_CONCURRENT: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnHudCaps {
    pub max_depth: u32,
    pub max_concurrent: u32,
}

impl Default for SpawnHudCaps {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_SPAWN_DEPTH,
            max_concurrent: DEFAULT_MAX_CONCURRENT,
        }
    }
}

impl SpawnHudCaps {
    pub fn from_config(max_subagents: u32) -> Self {
        let max_concurrent = if max_subagents > 0 {
            max_subagents.min(DEFAULT_MAX_CONCURRENT)
        } else {
            DEFAULT_MAX_CONCURRENT
        };
        Self {
            max_depth: DEFAULT_MAX_SPAWN_DEPTH,
            max_concurrent,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpawnHudMetrics {
    pub depth: u32,
    pub active: usize,
    pub widest_level: u64,
}

pub fn metrics_from_turn(state: &TurnActivityState) -> Option<SpawnHudMetrics> {
    if state.subagents.is_empty() {
        return None;
    }
    let rows: Vec<(u32, u32)> = state
        .subagents
        .values()
        .map(|r| (r.depth, r.task_index as u32))
        .collect();
    let depth = rows.iter().map(|(d, _)| *d).max().unwrap_or(0);
    let active = rows.len();
    let delegate_rows: Vec<crate::agents_overlay::DelegateRow> = state
        .subagents
        .values()
        .map(|r| crate::agents_overlay::DelegateRow {
            task_index: r.task_index,
            task_count: r.task_count,
            goal: r.goal.clone(),
            detail: r.detail.clone(),
            agent_id: r.agent_id.clone(),
            parent_id: r.parent_id.clone(),
            depth: r.depth,
            tool_count: r.tool_count,
            current_tool: r.current_tool.clone(),
            recent_tools: r.recent_tools.clone(),
            elapsed_secs: r.started_at.elapsed().as_secs(),
            started_at: r.started_at,
        })
        .collect();
    let widest_level = width_by_depth(&delegate_rows)
        .into_iter()
        .max()
        .unwrap_or(0);
    Some(SpawnHudMetrics {
        depth,
        active,
        widest_level,
    })
}

/// Returns HUD text like `│ d1/1 ⚡2/3+1` or None when no delegates are active.
pub fn format_spawn_hud(metrics: &SpawnHudMetrics, caps: &SpawnHudCaps) -> String {
    let depth_ratio = if caps.max_depth > 0 {
        metrics.depth as f64 / caps.max_depth as f64
    } else {
        0.0
    };
    let conc_ratio = if caps.max_concurrent > 0 {
        metrics.widest_level as f64 / caps.max_concurrent as f64
    } else {
        0.0
    };
    let at_cap = depth_ratio >= 1.0 || conc_ratio >= 1.0;
    let prefix = if at_cap { " │ ⚠ " } else { " │ " };

    let depth_label = if caps.max_depth > 0 {
        format!("d{}/{}", metrics.depth, caps.max_depth)
    } else {
        format!("d{}", metrics.depth)
    };

    let mut pieces = vec![depth_label];
    if metrics.active > 0 {
        let extra = metrics.active.saturating_sub(metrics.widest_level as usize);
        let width_label = if caps.max_concurrent > 0 {
            format!("{}/{}", metrics.widest_level, caps.max_concurrent)
        } else {
            metrics.widest_level.to_string()
        };
        let suffix = if extra > 0 {
            format!("+{extra}")
        } else {
            String::new()
        };
        pieces.push(format!("⚡{width_label}{suffix}"));
    }

    format!("{prefix}{}", pieces.join(" "))
}

/// Status-bar chip when global spawn pause is active (Hermes `delegation.pause`).
pub fn format_spawn_pause_chip() -> Option<&'static str> {
    if edgecrab_tools::delegation_state::is_spawn_paused() {
        Some(" ⏸ spawn paused")
    } else {
        None
    }
}

/// True when HUD should use warn/error coloring.
pub fn spawn_hud_severity(metrics: &SpawnHudMetrics, caps: &SpawnHudCaps) -> SpawnHudSeverity {
    let depth_ratio = if caps.max_depth > 0 {
        metrics.depth as f64 / caps.max_depth as f64
    } else {
        0.0
    };
    let conc_ratio = if caps.max_concurrent > 0 {
        metrics.widest_level as f64 / caps.max_concurrent as f64
    } else {
        0.0
    };
    let ratio = depth_ratio.max(conc_ratio);
    if ratio >= 1.0 {
        SpawnHudSeverity::Error
    } else if ratio >= 0.66 {
        SpawnHudSeverity::Warn
    } else {
        SpawnHudSeverity::Muted
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnHudSeverity {
    Muted,
    Warn,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turn_activity::TurnActivityState;

    #[test]
    fn hud_hidden_without_delegates() {
        let state = TurnActivityState::default();
        assert!(metrics_from_turn(&state).is_none());
    }

    #[test]
    fn hud_shows_depth_and_concurrency() {
        let mut state = TurnActivityState::default();
        state.enabled = true;
        state.on_subagent_start(0, 2, "a".into(), 1, "sa-0".into(), None);
        state.on_subagent_start(1, 2, "b".into(), 1, "sa-1".into(), None);
        let metrics = metrics_from_turn(&state).unwrap();
        let caps = SpawnHudCaps::default();
        let text = format_spawn_hud(&metrics, &caps);
        assert!(text.contains("d1/1"));
        assert!(text.contains('⚡'));
    }

    #[test]
    fn spawn_pause_chip_when_active() {
        edgecrab_tools::delegation_state::set_spawn_paused(true);
        assert_eq!(format_spawn_pause_chip(), Some(" ⏸ spawn paused"));
        edgecrab_tools::delegation_state::set_spawn_paused(false);
        assert_eq!(format_spawn_pause_chip(), None);
    }

    #[test]
    fn at_cap_prefix_warns() {
        let metrics = SpawnHudMetrics {
            depth: 1,
            active: 3,
            widest_level: 3,
        };
        let caps = SpawnHudCaps {
            max_depth: 1,
            max_concurrent: 3,
        };
        let text = format_spawn_hud(&metrics, &caps);
        assert!(text.contains('⚠'));
        assert_eq!(spawn_hud_severity(&metrics, &caps), SpawnHudSeverity::Error);
    }
}
