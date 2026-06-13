//! Subagent spawn tree helpers — Hermes `subagentTree.ts` parity.

use std::collections::{HashMap, HashSet};

use crate::agents_overlay::DelegateRow;

const ROOT_KEY: &str = "__root__";

/// Count delegates at each depth level (index 0 = top level).
pub fn width_by_depth(rows: &[DelegateRow]) -> Vec<u64> {
    let mut widths = Vec::new();
    for row in rows {
        let d = row.depth as usize;
        if widths.len() <= d {
            widths.resize(d + 1, 0);
        }
        widths[d] += 1;
    }
    widths
}

/// Depth-first display order (Hermes `buildSubagentTree` flatten for flat gateways).
pub fn sort_depth_first(rows: &mut [DelegateRow]) {
    sort_tree_depth_first(rows);
}

/// Parent-aware tree walk — groups by `parent_id`, then depth/index within siblings.
pub fn sort_tree_depth_first(rows: &mut [DelegateRow]) {
    if rows.is_empty() {
        return;
    }
    let known: HashSet<&str> = rows.iter().map(|r| r.agent_id.as_str()).collect();
    let mut by_parent: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        let parent = row
            .parent_id
            .as_deref()
            .filter(|p| known.contains(p))
            .unwrap_or(ROOT_KEY);
        by_parent.entry(parent).or_default().push(i);
    }
    for indices in by_parent.values_mut() {
        indices.sort_by_key(|&i| (rows[i].depth, rows[i].task_index));
    }
    let mut order = Vec::with_capacity(rows.len());
    walk_tree(&by_parent, ROOT_KEY, rows, &mut order);
    if order.len() == rows.len() {
        let sorted: Vec<DelegateRow> = order.into_iter().map(|i| rows[i].clone()).collect();
        rows.clone_from_slice(&sorted);
    } else {
        rows.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.task_index.cmp(&b.task_index)));
    }
}

fn walk_tree<'a>(
    by_parent: &HashMap<&'a str, Vec<usize>>,
    parent: &'a str,
    rows: &'a [DelegateRow],
    order: &mut Vec<usize>,
) {
    if let Some(indices) = by_parent.get(parent) {
        for &i in indices {
            order.push(i);
            walk_tree(by_parent, rows[i].agent_id.as_str(), rows, order);
        }
    }
}

/// Tree rail indent for overlay rows (`depth * 2` spaces).
pub fn tree_indent(depth: u32) -> String {
    "  ".repeat(depth as usize)
}

/// Sparkline from per-depth counts (uses shared shelf sparkline helper).
pub fn depth_sparkline(widths: &[u64]) -> String {
    if widths.len() < 2 {
        return String::new();
    }
    crate::shelf_visual::sparkline(widths)
}

/// Collect `root_id` and all descendant agent ids (for subtree interrupt).
pub fn descendant_agent_ids(rows: &[DelegateRow], root_id: &str) -> Vec<String> {
    let mut out = vec![root_id.to_string()];
    let mut queue = vec![root_id.to_string()];
    while let Some(parent) = queue.pop() {
        for row in rows {
            if row.parent_id.as_deref() == Some(parent.as_str()) && !out.contains(&row.agent_id) {
                out.push(row.agent_id.clone());
                queue.push(row.agent_id.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents_overlay::DelegateRow;

    fn row(task_index: usize, depth: u32, agent_id: &str, parent_id: Option<&str>) -> DelegateRow {
        DelegateRow {
            task_index,
            task_count: 3,
            goal: format!("goal {task_index}"),
            detail: None,
            tool_count: task_index,
            current_tool: None,
            recent_tools: Vec::new(),
            elapsed_secs: 1,
            depth,
            agent_id: agent_id.to_string(),
            parent_id: parent_id.map(str::to_string),
            started_at: std::time::Instant::now(),
        }
    }

    #[test]
    fn width_by_depth_counts_levels() {
        let rows = vec![
            row(0, 0, "sa-0", None),
            row(1, 0, "sa-1", None),
            row(0, 1, "sa-0/0", Some("sa-0")),
        ];
        assert_eq!(width_by_depth(&rows), vec![2, 1]);
    }

    #[test]
    fn tree_sort_groups_children_under_parent() {
        let mut rows = vec![
            row(0, 1, "sa-0/0", Some("sa-0")),
            row(0, 0, "sa-0", None),
            row(1, 0, "sa-1", None),
        ];
        sort_tree_depth_first(&mut rows);
        assert_eq!(rows[0].agent_id, "sa-0");
        assert_eq!(rows[1].agent_id, "sa-0/0");
        assert_eq!(rows[2].agent_id, "sa-1");
    }

    #[test]
    fn descendants_include_nested_children() {
        let rows = vec![
            row(0, 0, "sa-0", None),
            row(0, 1, "sa-0/0", Some("sa-0")),
            row(1, 0, "sa-1", None),
        ];
        let ids = descendant_agent_ids(&rows, "sa-0");
        assert_eq!(ids, vec!["sa-0".to_string(), "sa-0/0".to_string()]);
    }

    #[test]
    fn tree_indent_scales_with_depth() {
        assert_eq!(tree_indent(0), "");
        assert_eq!(tree_indent(2), "    ");
    }
}
