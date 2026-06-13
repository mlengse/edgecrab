//! Compact status summaries for background tasks and subagent delegates.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BackgroundTaskStatus {
    pub preview: String,
    pub last_progress: Option<String>,
    pub last_seq: u64,
}

#[derive(Clone, Debug)]
pub struct ActiveSubagentStatus {
    pub task_index: usize,
    pub task_count: usize,
    pub goal: String,
    pub last_detail: Option<String>,
    pub last_seq: u64,
}

pub fn format_background_status_summary(
    active: &HashMap<String, BackgroundTaskStatus>,
) -> Option<String> {
    let current = active.values().max_by_key(|status| status.last_seq)?;
    let detail = current
        .last_progress
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or(&current.preview);
    Some(edgecrab_core::safe_truncate(detail, 58).to_string())
}

pub fn format_subagent_status_summary(
    active: &HashMap<usize, ActiveSubagentStatus>,
) -> Option<String> {
    let current = active.values().max_by_key(|status| status.last_seq)?;
    let detail = current
        .last_detail
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .map(|text| edgecrab_core::safe_truncate(text, 52).to_string())
        .unwrap_or_else(|| edgecrab_core::safe_truncate(&current.goal, 52).to_string());
    Some(format!(
        "[{}/{}] {}",
        current.task_index + 1,
        current.task_count,
        detail
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subagent_summary_picks_latest_seq() {
        let mut map = HashMap::new();
        map.insert(
            0,
            ActiveSubagentStatus {
                task_index: 0,
                task_count: 2,
                goal: "older".into(),
                last_detail: None,
                last_seq: 1,
            },
        );
        map.insert(
            1,
            ActiveSubagentStatus {
                task_index: 1,
                task_count: 2,
                goal: "newer goal".into(),
                last_detail: Some("running grep".into()),
                last_seq: 9,
            },
        );
        let summary = format_subagent_status_summary(&map).unwrap();
        assert!(summary.contains("2/2"));
        assert!(summary.contains("running grep"));
    }
}
