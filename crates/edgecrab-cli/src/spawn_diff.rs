//! Spawn turn diff — Hermes `agentsOverlay.tsx` DiffView MVP (metric deltas).

use crate::spawn_history::SpawnTurnSnapshot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffMetric {
    pub label: &'static str,
    pub baseline: String,
    pub candidate: String,
    pub delta: String,
}

pub fn diff_turn_snapshots(
    baseline: &SpawnTurnSnapshot,
    candidate: &SpawnTurnSnapshot,
) -> Vec<DiffMetric> {
    let base_ok = count_delegate_status(baseline, "completed");
    let cand_ok = count_delegate_status(candidate, "completed");
    let base_err = count_delegate_status(baseline, "error");
    let cand_err = count_delegate_status(candidate, "error");
    vec![
        metric_usize(
            "delegates",
            baseline.delegate_count(),
            candidate.delegate_count(),
        ),
        metric_usize("tool calls", baseline.total_tools, candidate.total_tools),
        metric_u64(
            "max duration",
            baseline.total_duration_secs,
            candidate.total_duration_secs,
            fmt_secs,
        ),
        metric_usize("fan-out", baseline.max_fanout, candidate.max_fanout),
        metric_u32("tokens (est)", baseline.token_est, candidate.token_est),
        metric_f64("cost", baseline.cost_usd, candidate.cost_usd),
        DiffMetric {
            label: "completed",
            baseline: base_ok.to_string(),
            candidate: cand_ok.to_string(),
            delta: signed_delta(cand_ok as i64 - base_ok as i64),
        },
        DiffMetric {
            label: "errors",
            baseline: base_err.to_string(),
            candidate: cand_err.to_string(),
            delta: signed_delta(cand_err as i64 - base_err as i64),
        },
    ]
}

fn count_delegate_status(snapshot: &SpawnTurnSnapshot, status: &str) -> usize {
    snapshot
        .delegates
        .iter()
        .filter(|d| d.status.eq_ignore_ascii_case(status))
        .count()
}

/// Goal-level delegate diff — goals in candidate not present in baseline label set.
pub fn diff_delegate_goals(
    baseline: &SpawnTurnSnapshot,
    candidate: &SpawnTurnSnapshot,
) -> Vec<String> {
    let base_goals: std::collections::HashSet<&str> =
        baseline.delegates.iter().map(|d| d.goal.as_str()).collect();
    candidate
        .delegates
        .iter()
        .filter(|d| !base_goals.contains(d.goal.as_str()))
        .map(|d| format!("+ {}", d.goal))
        .take(6)
        .collect()
}

/// Goals removed in candidate vs baseline.
pub fn diff_delegate_goals_removed(
    baseline: &SpawnTurnSnapshot,
    candidate: &SpawnTurnSnapshot,
) -> Vec<String> {
    let cand_goals: std::collections::HashSet<&str> = candidate
        .delegates
        .iter()
        .map(|d| d.goal.as_str())
        .collect();
    baseline
        .delegates
        .iter()
        .filter(|d| !cand_goals.contains(d.goal.as_str()))
        .map(|d| format!("− {}", d.goal))
        .take(6)
        .collect()
}

fn metric_u32(label: &'static str, base: u32, cand: u32) -> DiffMetric {
    DiffMetric {
        label,
        baseline: fmt_token_est(base),
        candidate: fmt_token_est(cand),
        delta: signed_delta(cand as i64 - base as i64),
    }
}

fn metric_f64(label: &'static str, base: f64, cand: f64) -> DiffMetric {
    DiffMetric {
        label,
        baseline: fmt_cost(base),
        candidate: fmt_cost(cand),
        delta: signed_cost_delta(cand - base),
    }
}

fn fmt_token_est(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub fn fmt_cost(usd: f64) -> String {
    if usd >= 1.0 {
        format!("${usd:.2}")
    } else if usd >= 0.0001 {
        format!("${usd:.4}")
    } else {
        "$0.00".into()
    }
}

fn signed_cost_delta(delta: f64) -> String {
    if delta.abs() < 0.000_05 {
        "±0".into()
    } else if delta > 0.0 {
        format!("+{}", fmt_cost(delta).trim_start_matches('$'))
    } else {
        format!("-{}", fmt_cost(-delta).trim_start_matches('$'))
    }
}

fn metric_usize(label: &'static str, base: usize, cand: usize) -> DiffMetric {
    DiffMetric {
        label,
        baseline: base.to_string(),
        candidate: cand.to_string(),
        delta: signed_delta(cand as i64 - base as i64),
    }
}

fn metric_u64(label: &'static str, base: u64, cand: u64, fmt: fn(u64) -> String) -> DiffMetric {
    DiffMetric {
        label,
        baseline: fmt(base),
        candidate: fmt(cand),
        delta: signed_delta(cand as i64 - base as i64),
    }
}

fn fmt_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}

fn signed_delta(delta: i64) -> String {
    if delta > 0 {
        format!("+{delta}")
    } else if delta < 0 {
        delta.to_string()
    } else {
        "±0".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawn_history::{SpawnHistoryEntry, TurnCommitMetrics};

    fn snap(label: &str, delegates: usize, tools: usize, duration: u64) -> SpawnTurnSnapshot {
        let entries: Vec<SpawnHistoryEntry> = (0..delegates)
            .map(|i| SpawnHistoryEntry {
                task_index: i,
                task_count: delegates,
                goal: format!("goal-{i}"),
                agent_id: format!("sa-{i}"),
                parent_id: None,
                depth: 1,
                tool_count: tools / delegates.max(1),
                duration_secs: duration / delegates.max(1) as u64,
                status: "completed".into(),
            })
            .collect();
        SpawnTurnSnapshot::from_entries(label.into(), entries, TurnCommitMetrics::default())
    }

    fn snap_with_metrics(
        label: &str,
        delegates: usize,
        tools: usize,
        duration: u64,
        token_est: u32,
        cost_usd: f64,
    ) -> SpawnTurnSnapshot {
        let mut snap = snap(label, delegates, tools, duration);
        snap.token_est = token_est;
        snap.cost_usd = cost_usd;
        snap
    }

    #[test]
    fn diff_shows_positive_tool_delta() {
        let base = snap("run-a", 2, 4, 30);
        let cand = snap("run-b", 2, 10, 45);
        let metrics = diff_turn_snapshots(&base, &cand);
        let tools = metrics.iter().find(|m| m.label == "tool calls").unwrap();
        assert_eq!(tools.delta, "+6");
    }

    #[test]
    fn summary_metrics_cover_delegates_and_tools() {
        let base = snap("run-a", 2, 4, 30);
        let cand = snap("run-b", 2, 10, 45);
        let metrics = diff_turn_snapshots(&base, &cand);
        assert_eq!(metrics.len(), 8);
        assert!(metrics.iter().any(|m| m.label == "delegates"));
        assert!(metrics.iter().any(|m| m.label == "tool calls"));
        assert!(metrics.iter().any(|m| m.label == "completed"));
        assert!(metrics.iter().any(|m| m.label == "errors"));
    }

    #[test]
    fn diff_shows_token_and_cost_deltas() {
        let base = snap_with_metrics("run-a", 2, 4, 30, 1200, 0.05);
        let cand = snap_with_metrics("run-b", 2, 10, 45, 4800, 0.22);
        let metrics = diff_turn_snapshots(&base, &cand);
        let tokens = metrics.iter().find(|m| m.label == "tokens (est)").unwrap();
        assert_eq!(tokens.delta, "+3600");
        let cost = metrics.iter().find(|m| m.label == "cost").unwrap();
        assert!(cost.delta.starts_with('+'));
    }

    #[test]
    fn delegate_goal_diff_lists_new_goals() {
        let mut base = snap("run-a", 1, 2, 10);
        base.delegates[0].goal = "scan repo".into();
        let mut cand = snap("run-b", 2, 4, 20);
        cand.delegates[0].goal = "scan repo".into();
        cand.delegates
            .push(crate::spawn_history::SpawnHistoryEntry {
                task_index: 1,
                task_count: 2,
                goal: "write tests".into(),
                agent_id: "sa-1".into(),
                parent_id: None,
                depth: 1,
                tool_count: 2,
                duration_secs: 15,
                status: "completed".into(),
            });
        let added = diff_delegate_goals(&base, &cand);
        assert!(added.iter().any(|g| g.contains("write tests")));
    }
}
