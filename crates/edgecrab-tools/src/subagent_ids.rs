//! Stable subagent identifiers — Hermes `SubagentProgress.id` parity.

/// Root-batch delegate id (`sa-0`, `sa-1`, …).
pub fn subagent_agent_id(task_index: usize) -> String {
    format!("sa-{task_index}")
}

/// Nested delegate id under a parent subagent (`sa-0/0`, `sa-0/1`, …).
pub fn nested_subagent_agent_id(parent_id: &str, task_index: usize) -> String {
    format!("{parent_id}/{task_index}")
}

/// Resolve stable id for a spawn given optional parent context.
pub fn resolve_subagent_agent_id(parent_agent_id: Option<&str>, task_index: usize) -> String {
    match parent_agent_id {
        Some(parent) => nested_subagent_agent_id(parent, task_index),
        None => subagent_agent_id(task_index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_and_nested_ids() {
        assert_eq!(subagent_agent_id(2), "sa-2");
        assert_eq!(nested_subagent_agent_id("sa-0", 1), "sa-0/1");
        assert_eq!(resolve_subagent_agent_id(Some("sa-0"), 0), "sa-0/0");
        assert_eq!(resolve_subagent_agent_id(None, 3), "sa-3");
    }
}
