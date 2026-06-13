//! `/agents` overlay — spawn-tree dashboard (Hermes `agentsOverlay.tsx` parity).

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::gantt_strip::{build_gantt_spans, render_gantt_lines};
use crate::overlay_layout::popup_rect;
use crate::shelf_visual::{
    delegate_status_glyph, elapsed_heat, fmt_duration, format_recent_tools, heat_color, sparkline,
};
use crate::spawn_diff::{diff_delegate_goals, diff_delegate_goals_removed, diff_turn_snapshots};
use crate::spawn_history::{SpawnHistory, SpawnHistoryEntry, SpawnTurnSnapshot};
use crate::theme::Theme;
use crate::turn_activity::{ShelfSubagentRow, TurnActivityState};

/// Sort modes — Hermes `agentsOverlay.tsx` `SORT_ORDER`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentsSortMode {
    #[default]
    SpawnOrder,
    DepthFirst,
    ToolCountDesc,
    DurationDesc,
}

impl AgentsSortMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::SpawnOrder => "spawn order",
            Self::DepthFirst => "depth-first",
            Self::ToolCountDesc => "busiest",
            Self::DurationDesc => "slowest",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::SpawnOrder => Self::DepthFirst,
            Self::DepthFirst => Self::ToolCountDesc,
            Self::ToolCountDesc => Self::DurationDesc,
            Self::DurationDesc => Self::SpawnOrder,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DelegateRow {
    pub task_index: usize,
    pub task_count: usize,
    pub goal: String,
    pub detail: Option<String>,
    pub agent_id: String,
    pub parent_id: Option<String>,
    pub depth: u32,
    pub tool_count: usize,
    pub current_tool: Option<String>,
    pub recent_tools: Vec<String>,
    pub elapsed_secs: u64,
    pub started_at: Instant,
}

impl DelegateRow {
    pub fn from_shelf(row: &ShelfSubagentRow, now: Instant) -> Self {
        Self::from_shelf_row(row, now)
    }

    pub fn from_history(entry: &SpawnHistoryEntry) -> Self {
        Self {
            task_index: entry.task_index,
            task_count: entry.task_count,
            goal: entry.goal.clone(),
            detail: None,
            agent_id: entry.agent_id.clone(),
            parent_id: entry.parent_id.clone(),
            depth: entry.depth,
            tool_count: entry.tool_count,
            current_tool: None,
            recent_tools: Vec::new(),
            elapsed_secs: entry.duration_secs,
            started_at: Instant::now(),
        }
    }

    fn from_shelf_row(row: &ShelfSubagentRow, now: Instant) -> Self {
        Self {
            task_index: row.task_index,
            task_count: row.task_count,
            goal: row.goal.clone(),
            detail: row.detail.clone(),
            agent_id: row.agent_id.clone(),
            parent_id: row.parent_id.clone(),
            depth: row.depth,
            tool_count: row.tool_count,
            current_tool: row.current_tool.clone(),
            recent_tools: row.recent_tools.clone(),
            elapsed_secs: now
                .checked_duration_since(row.started_at)
                .unwrap_or(Duration::ZERO)
                .as_secs(),
            started_at: row.started_at,
        }
    }
}

pub fn build_delegate_rows(state: &TurnActivityState, now: Instant) -> Vec<DelegateRow> {
    let mut rows: Vec<DelegateRow> = state
        .subagents
        .values()
        .map(|row| DelegateRow::from_shelf(row, now))
        .collect();
    sort_delegate_rows(&mut rows);
    rows
}

pub fn build_delegate_rows_from_snapshot(snapshot: &SpawnTurnSnapshot) -> Vec<DelegateRow> {
    let mut rows: Vec<DelegateRow> = snapshot
        .delegates
        .iter()
        .map(DelegateRow::from_history)
        .collect();
    sort_delegate_rows(&mut rows);
    rows
}

pub fn sort_delegate_rows(rows: &mut [DelegateRow]) {
    rows.sort_by_key(|r| r.task_index);
}

pub fn sort_delegate_rows_by(rows: &mut [DelegateRow], mode: AgentsSortMode) {
    match mode {
        AgentsSortMode::SpawnOrder => rows.sort_by_key(|r| r.task_index),
        AgentsSortMode::DepthFirst => crate::subagent_tree::sort_depth_first(rows),
        AgentsSortMode::ToolCountDesc => rows.sort_by(|a, b| {
            b.tool_count
                .cmp(&a.tool_count)
                .then(a.task_index.cmp(&b.task_index))
        }),
        AgentsSortMode::DurationDesc => rows.sort_by(|a, b| {
            b.elapsed_secs
                .cmp(&a.elapsed_secs)
                .then(a.task_index.cmp(&b.task_index))
        }),
    }
}

#[derive(Clone, Debug, Default)]
pub struct AgentsOverlay {
    pub active: bool,
    pub cursor: usize,
    pub sort: AgentsSortMode,
    /// Side-by-side diff of the two most recent turn snapshots.
    pub show_diff: bool,
    /// 0 = live turn; 1..N = Nth-most-recent archived snapshot (Hermes history nav).
    pub history_index: usize,
}

impl AgentsOverlay {
    pub fn open(&mut self) {
        self.active = true;
        self.cursor = 0;
        self.show_diff = false;
        self.history_index = 0;
    }

    pub fn open_replay(&mut self, history_index: usize) {
        self.active = true;
        self.cursor = 0;
        self.show_diff = false;
        self.history_index = history_index.max(1);
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

pub enum AgentsOverlayAction {
    None,
    Close,
    SendStopSteer,
    InterruptSubagent(String),
    InterruptSubtree(String),
    ToggleSpawnPause,
    Refresh,
    ToggleDiff,
    HistoryChanged,
}

pub fn handle_agents_overlay_key(
    overlay: &mut AgentsOverlay,
    key: KeyEvent,
    row_count: usize,
    turn_count: usize,
    selected_agent_id: Option<&str>,
    live_mode: bool,
) -> AgentsOverlayAction {
    if !overlay.active {
        return AgentsOverlayAction::None;
    }

    if overlay.show_diff {
        return match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) | (_, KeyCode::Char('d')) => {
                overlay.show_diff = false;
                AgentsOverlayAction::Refresh
            }
            _ => AgentsOverlayAction::None,
        };
    }

    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
            overlay.close();
            AgentsOverlayAction::Close
        }
        (_, KeyCode::Tab) | (_, KeyCode::Char('s')) if key.modifiers.is_empty() => {
            overlay.sort = overlay.sort.cycle();
            overlay.cursor = 0;
            AgentsOverlayAction::Refresh
        }
        (_, KeyCode::Char('d')) if turn_count >= 2 => {
            overlay.show_diff = true;
            AgentsOverlayAction::ToggleDiff
        }
        (_, KeyCode::Char('[')) if turn_count > 0 => {
            overlay.history_index = (overlay.history_index + 1).min(turn_count);
            overlay.cursor = 0;
            AgentsOverlayAction::HistoryChanged
        }
        (_, KeyCode::Char(']')) if overlay.history_index > 0 => {
            overlay.history_index = overlay.history_index.saturating_sub(1);
            overlay.cursor = 0;
            AgentsOverlayAction::HistoryChanged
        }
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
            if row_count > 0 {
                overlay.cursor = overlay.cursor.checked_sub(1).unwrap_or(row_count - 1);
            }
            AgentsOverlayAction::None
        }
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
            if row_count > 0 {
                overlay.cursor = (overlay.cursor + 1) % row_count;
            }
            AgentsOverlayAction::None
        }
        (_, KeyCode::Char('x')) if live_mode && selected_agent_id.is_some() => {
            AgentsOverlayAction::InterruptSubagent(selected_agent_id.unwrap().to_string())
        }
        (_, KeyCode::Char('X')) if live_mode && selected_agent_id.is_some() => {
            AgentsOverlayAction::InterruptSubtree(selected_agent_id.unwrap().to_string())
        }
        (_, KeyCode::Char('p')) if live_mode => AgentsOverlayAction::ToggleSpawnPause,
        (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Char('i')) => {
            AgentsOverlayAction::SendStopSteer
        }
        _ => AgentsOverlayAction::None,
    }
}

pub struct AgentsRenderParams<'a> {
    pub overlay: &'a AgentsOverlay,
    pub rows: &'a [DelegateRow],
    pub history: &'a SpawnHistory,
    pub theme: &'a Theme,
    pub total_tool_calls: usize,
}

pub fn render_agents_overlay(frame: &mut Frame, area: Rect, params: &AgentsRenderParams<'_>) {
    if !params.overlay.active {
        return;
    }

    frame.render_widget(Clear, area);

    let accent = params
        .theme
        .shelf_accent
        .fg
        .unwrap_or(Color::Rgb(205, 175, 50));
    let dim = params.theme.shelf_dim.fg.unwrap_or(Color::DarkGray);
    let warn = params.theme.shelf_hint.fg.unwrap_or(Color::Yellow);
    let hot = params.theme.output_error.fg.unwrap_or(Color::Red);
    let border = params.theme.shelf_border;

    let pw = (area.width * 92 / 100).max(24);
    let ph = (area.height * 88 / 100).max(10);
    let popup = popup_rect(area, pw, ph);

    if params.overlay.show_diff {
        render_diff_view(frame, popup, params);
        return;
    }

    let recent_turns: Vec<&SpawnTurnSnapshot> = params.history.turns().take(3).collect();
    let gantt_spans = build_gantt_spans(params.rows);
    let gantt_height = if params.rows.len() >= 2 && !gantt_spans.is_empty() {
        6u16
    } else {
        0
    };
    let history_constraint = if recent_turns.is_empty() {
        Constraint::Length(0)
    } else {
        Constraint::Length((recent_turns.len() as u16).saturating_add(2))
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(gantt_height),
            Constraint::Min(4),
            history_constraint,
            Constraint::Length(1),
        ])
        .split(popup);

    let depth_spark = {
        let widths = crate::subagent_tree::width_by_depth(params.rows);
        let s = crate::subagent_tree::depth_sparkline(&widths);
        if s.is_empty() {
            String::new()
        } else {
            format!(" · depth {s}")
        }
    };
    let spark = if params.rows.len() >= 2 {
        let counts: Vec<u64> = params.rows.iter().map(|r| r.tool_count as u64).collect();
        let s = sparkline(&counts);
        if s.is_empty() {
            depth_spark
        } else {
            format!(" · {s}{depth_spark}")
        }
    } else {
        depth_spark
    };

    let live_label = if params.overlay.history_index > 0 {
        format!(
            "replay {}/{}",
            params.overlay.history_index,
            params.history.turn_count()
        )
    } else {
        let pause = edgecrab_tools::delegation_state::is_spawn_paused();
        format!(
            "{} live{}",
            params.rows.len(),
            if pause { " · ⏸ paused" } else { "" }
        )
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " agents ",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "· {live_label} · {} tool call(s){spark}",
                    params.total_tool_calls
                ),
                Style::default().fg(dim),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" sort: {} ", params.overlay.sort.label()),
                Style::default().fg(dim).add_modifier(Modifier::ITALIC),
            ),
            Span::styled("Tab", Style::default().fg(accent)),
            Span::styled(" cycle", Style::default().fg(dim)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border)),
    );
    frame.render_widget(header, chunks[0]);

    if gantt_height > 0 {
        let gantt_lines = render_gantt_lines(
            &gantt_spans,
            popup.width,
            params.overlay.cursor,
            accent,
            dim,
            4,
        );
        let gantt = Paragraph::new(gantt_lines).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(border)),
        );
        frame.render_widget(gantt, chunks[1]);
    }

    let list_chunk = if gantt_height > 0 {
        chunks[2]
    } else {
        chunks[1]
    };
    let history_chunk = if gantt_height > 0 {
        chunks[3]
    } else {
        chunks[2]
    };
    let footer_chunk = if gantt_height > 0 {
        chunks[4]
    } else {
        chunks[3]
    };

    if params.rows.is_empty() {
        let mut empty_lines = vec![
            Line::from(Span::styled(
                "No active delegates",
                Style::default().fg(dim).add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                "  delegate_task fans out parallel work",
                Style::default().fg(dim),
            )),
        ];
        if params.history.turn_count() >= 2 {
            empty_lines.push(Line::from(Span::styled(
                "  d — compare last two turn snapshots",
                Style::default().fg(accent),
            )));
        }
        let empty = Paragraph::new(empty_lines).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(border)),
        );
        frame.render_widget(empty, list_chunk);
    } else {
        let items: Vec<ListItem> = params
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                delegate_list_item(row, i, params.overlay.cursor, accent, dim, warn, hot)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(border)),
        );
        frame.render_widget(list, list_chunk);
    }

    if !recent_turns.is_empty() {
        let history_lines: Vec<Line> = recent_turns
            .iter()
            .map(|turn| {
                Line::from(vec![
                    Span::styled(" ◷ ", Style::default().fg(accent)),
                    Span::styled(turn.label.clone(), Style::default().fg(dim)),
                    Span::styled(
                        format!(
                            " · {} del · {} tools · {}",
                            turn.delegate_count(),
                            turn.total_tools,
                            fmt_duration(turn.total_duration_secs)
                        ),
                        Style::default().fg(dim).add_modifier(Modifier::ITALIC),
                    ),
                ])
            })
            .collect();
        let mut history_block_lines = vec![Line::from(Span::styled(
            " recent turns ",
            Style::default().fg(dim).add_modifier(Modifier::BOLD),
        ))];
        history_block_lines.extend(history_lines);
        let history_block = Paragraph::new(history_block_lines).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(border)),
        );
        frame.render_widget(history_block, history_chunk);
    }

    let mut footer_spans = vec![
        Span::styled(" ↑↓ ", Style::default().fg(accent)),
        Span::styled("select  ", Style::default().fg(dim)),
        Span::styled("Tab ", Style::default().fg(accent)),
        Span::styled("sort  ", Style::default().fg(dim)),
        Span::styled("i ", Style::default().fg(accent)),
        Span::styled("STOP  ", Style::default().fg(dim)),
    ];
    if params.overlay.history_index == 0 && !params.rows.is_empty() {
        let pause = edgecrab_tools::delegation_state::is_spawn_paused();
        footer_spans.extend([
            Span::styled("x ", Style::default().fg(accent)),
            Span::styled("kill  ", Style::default().fg(dim)),
            Span::styled("X ", Style::default().fg(accent)),
            Span::styled("subtree  ", Style::default().fg(dim)),
            Span::styled("p ", Style::default().fg(accent)),
            Span::styled(
                if pause { "resume  " } else { "pause  " },
                Style::default().fg(dim),
            ),
        ]);
    }
    if params.history.turn_count() >= 2 {
        footer_spans.extend([
            Span::styled("d ", Style::default().fg(accent)),
            Span::styled("diff  ", Style::default().fg(dim)),
        ]);
    }
    if params.history.turn_count() > 0 {
        footer_spans.extend([
            Span::styled("[/] ", Style::default().fg(accent)),
            Span::styled("history  ", Style::default().fg(dim)),
        ]);
    }
    footer_spans.extend([
        Span::styled("Esc ", Style::default().fg(accent)),
        Span::styled("close", Style::default().fg(dim)),
    ]);
    frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer_chunk);
}

fn delegate_list_item(
    row: &DelegateRow,
    index: usize,
    cursor: usize,
    accent: Color,
    dim: Color,
    warn: Color,
    hot: Color,
) -> ListItem<'static> {
    let heat = elapsed_heat(row.elapsed_secs);
    let elapsed_style = Style::default().fg(heat_color(heat, dim, warn, hot));
    let selected = index == cursor;
    let prefix = if selected { "▸ " } else { "  " };
    let indent = crate::subagent_tree::tree_indent(row.depth);
    let tool_tail = format_recent_tools(&row.recent_tools, 3);
    let tool_suffix = if row.tool_count > 0 {
        format!(" · {} tools", row.tool_count)
    } else {
        String::new()
    };
    let detail = row
        .detail
        .as_deref()
        .or(row.current_tool.as_deref())
        .map(|d| format!(" · {d}"))
        .unwrap_or_default();
    ListItem::new(vec![
        Line::from(vec![
            Span::styled(prefix, Style::default().fg(accent)),
            Span::styled(
                format!(
                    "{indent}[{}/{}] {}",
                    row.task_index + 1,
                    row.task_count,
                    row.goal
                ),
                if selected {
                    Style::default().fg(accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(dim)
                },
            ),
            Span::styled(
                detail,
                Style::default().fg(dim).add_modifier(Modifier::ITALIC),
            ),
            Span::styled(tool_suffix, Style::default().fg(dim)),
            Span::styled(
                format!(" · {}", fmt_duration(row.elapsed_secs)),
                elapsed_style,
            ),
        ]),
        Line::from(Span::styled(
            format!("     {tool_tail}"),
            Style::default().fg(dim).add_modifier(Modifier::DIM),
        )),
    ])
    .style(if selected {
        Style::default().bg(Color::Rgb(30, 30, 36))
    } else {
        Style::default()
    })
}

fn render_diff_view(frame: &mut Frame, popup: Rect, params: &AgentsRenderParams<'_>) {
    let accent = params
        .theme
        .shelf_accent
        .fg
        .unwrap_or(Color::Rgb(205, 175, 50));
    let dim = params.theme.shelf_dim.fg.unwrap_or(Color::DarkGray);
    let ok = Color::Rgb(120, 200, 120);
    let hot = params.theme.output_error.fg.unwrap_or(Color::Red);
    let border = params.theme.shelf_border;

    let turns: Vec<&SpawnTurnSnapshot> = params.history.turns().take(2).collect();
    if turns.len() < 2 {
        return;
    }
    let candidate = turns[0];
    let baseline = turns[1];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(popup);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            " spawn diff ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " side-by-side · newest vs previous turn",
            Style::default().fg(dim).add_modifier(Modifier::ITALIC),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border)),
    );
    frame.render_widget(header, chunks[0]);

    let mut delta_lines = vec![Line::from(Span::styled(
        " Δ metrics ",
        Style::default().fg(dim).add_modifier(Modifier::BOLD),
    ))];
    for metric in diff_turn_snapshots(baseline, candidate) {
        let delta_style = if metric.delta.starts_with('+') {
            Style::default().fg(hot)
        } else if metric.delta.starts_with('-') {
            Style::default().fg(ok)
        } else {
            Style::default().fg(dim)
        };
        delta_lines.push(Line::from(vec![
            Span::styled(format!("{:<12} ", metric.label), Style::default().fg(dim)),
            Span::styled(
                format!("{} → {}", metric.baseline, metric.candidate),
                Style::default().fg(dim),
            ),
            Span::styled(format!(" ({})", metric.delta), delta_style),
        ]));
    }
    let added = diff_delegate_goals(baseline, candidate);
    let removed = diff_delegate_goals_removed(baseline, candidate);
    if !added.is_empty() || !removed.is_empty() {
        delta_lines.push(Line::from(Span::styled(
            format!(" goals +{} −{}", added.len(), removed.len()),
            Style::default().fg(dim).add_modifier(Modifier::ITALIC),
        )));
    }
    frame.render_widget(
        Paragraph::new(delta_lines).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(border)),
        ),
        chunks[1],
    );

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    render_diff_turn_pane(frame, panes[0], "baseline", baseline, dim, border, false);
    render_diff_turn_pane(
        frame,
        panes[1],
        "candidate",
        candidate,
        accent,
        border,
        true,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" d/Esc ", Style::default().fg(accent)),
            Span::styled("back", Style::default().fg(dim)),
        ])),
        chunks[3],
    );
}

fn render_diff_turn_pane(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    snapshot: &SpawnTurnSnapshot,
    accent: Color,
    border: Color,
    emphasize: bool,
) {
    let dim = Color::DarkGray;
    let mut lines = vec![
        Line::from(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            snapshot.label.clone(),
            Style::default().fg(if emphasize { accent } else { dim }),
        )),
        Line::from(Span::styled(
            format!(
                " {} del · {} tools · {} · ${:.4}",
                snapshot.delegate_count(),
                snapshot.total_tools,
                fmt_duration(snapshot.total_duration_secs),
                snapshot.cost_usd
            ),
            Style::default().fg(dim).add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " delegates ",
            Style::default().fg(dim).add_modifier(Modifier::BOLD),
        )),
    ];
    for entry in snapshot.delegates.iter().take(5) {
        let glyph = delegate_status_glyph(&entry.status);
        lines.push(Line::from(vec![
            Span::styled(format!(" {glyph} "), Style::default().fg(accent)),
            Span::styled(
                edgecrab_core::safe_truncate(&entry.goal, 42).to_string(),
                Style::default().fg(dim),
            ),
            Span::styled(
                format!(" · {}t", entry.tool_count),
                Style::default().fg(dim).add_modifier(Modifier::DIM),
            ),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border)),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawn_history::{SpawnHistoryEntry, SpawnTurnSnapshot};

    fn sample_row(index: usize, tools: usize, elapsed: u64) -> DelegateRow {
        DelegateRow {
            task_index: index,
            task_count: 3,
            depth: 1,
            goal: format!("goal-{index}"),
            detail: None,
            agent_id: format!("sa-{index}"),
            parent_id: None,
            tool_count: tools,
            current_tool: None,
            recent_tools: vec!["file_read".into()],
            elapsed_secs: elapsed,
            started_at: std::time::Instant::now(),
        }
    }

    #[test]
    fn sort_busiest_first() {
        let mut rows = vec![
            sample_row(0, 1, 5),
            sample_row(1, 9, 2),
            sample_row(2, 3, 8),
        ];
        sort_delegate_rows_by(&mut rows, AgentsSortMode::ToolCountDesc);
        assert_eq!(rows[0].task_index, 1);
    }

    #[test]
    fn sort_slowest_first() {
        let mut rows = vec![
            sample_row(0, 1, 5),
            sample_row(1, 1, 40),
            sample_row(2, 1, 10),
        ];
        sort_delegate_rows_by(&mut rows, AgentsSortMode::DurationDesc);
        assert_eq!(rows[0].task_index, 1);
    }

    #[test]
    fn sort_depth_first() {
        let mut rows = vec![
            DelegateRow {
                task_index: 2,
                task_count: 3,
                depth: 1,
                goal: "deep".into(),
                detail: None,
                agent_id: "sa-2".into(),
                parent_id: Some("sa-0".into()),
                tool_count: 0,
                current_tool: None,
                recent_tools: Vec::new(),
                elapsed_secs: 1,
                started_at: std::time::Instant::now(),
            },
            DelegateRow {
                task_index: 1,
                task_count: 3,
                depth: 0,
                goal: "root".into(),
                detail: None,
                agent_id: "sa-0".into(),
                parent_id: None,
                tool_count: 0,
                current_tool: None,
                recent_tools: Vec::new(),
                elapsed_secs: 1,
                started_at: std::time::Instant::now(),
            },
        ];
        sort_delegate_rows_by(&mut rows, AgentsSortMode::DepthFirst);
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[1].depth, 1);
    }

    #[test]
    fn overlay_esc_closes() {
        let mut overlay = AgentsOverlay {
            active: true,
            ..Default::default()
        };
        let action = handle_agents_overlay_key(
            &mut overlay,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            1,
            0,
            Some("sa-0"),
            true,
        );
        assert!(matches!(action, AgentsOverlayAction::Close));
        assert!(!overlay.active);
    }

    #[test]
    fn diff_requires_two_turns() {
        let mut overlay = AgentsOverlay {
            active: true,
            ..Default::default()
        };
        let action = handle_agents_overlay_key(
            &mut overlay,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            0,
            1,
            None,
            true,
        );
        assert!(matches!(action, AgentsOverlayAction::None));
        assert!(!overlay.show_diff);

        let action = handle_agents_overlay_key(
            &mut overlay,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            0,
            2,
            None,
            true,
        );
        assert!(matches!(action, AgentsOverlayAction::ToggleDiff));
        assert!(overlay.show_diff);
    }

    #[test]
    fn overlay_x_interrupts_selected_subagent() {
        let mut overlay = AgentsOverlay {
            active: true,
            ..Default::default()
        };
        let action = handle_agents_overlay_key(
            &mut overlay,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            2,
            0,
            Some("sa-1"),
            true,
        );
        assert!(matches!(action, AgentsOverlayAction::InterruptSubagent(id) if id == "sa-1"));
    }

    #[test]
    fn overlay_p_toggles_spawn_pause_action() {
        let mut overlay = AgentsOverlay {
            active: true,
            ..Default::default()
        };
        let action = handle_agents_overlay_key(
            &mut overlay,
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
            1,
            0,
            None,
            true,
        );
        assert!(matches!(action, AgentsOverlayAction::ToggleSpawnPause));
    }

    #[test]
    fn overlay_interrupt_disabled_in_replay() {
        let mut overlay = AgentsOverlay {
            active: true,
            history_index: 1,
            ..Default::default()
        };
        let action = handle_agents_overlay_key(
            &mut overlay,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            2,
            1,
            Some("sa-0"),
            false,
        );
        assert!(matches!(action, AgentsOverlayAction::None));
    }

    #[test]
    fn build_rows_from_turn_activity() {
        let mut state = TurnActivityState::new(true);
        state.on_subagent_start(1, 2, "scan repo".into(), 1, "sa-1".into(), None);
        let rows = build_delegate_rows(&state, Instant::now());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].goal, "scan repo");
    }

    #[test]
    fn turn_snapshot_label_from_history() {
        let turn = SpawnTurnSnapshot::from_entries(
            "a · b".into(),
            vec![
                SpawnHistoryEntry {
                    task_index: 0,
                    task_count: 2,
                    goal: "a".into(),
                    agent_id: "sa-0".into(),
                    parent_id: None,
                    depth: 1,
                    tool_count: 1,
                    duration_secs: 5,
                    status: "completed".into(),
                },
                SpawnHistoryEntry {
                    task_index: 1,
                    task_count: 2,
                    goal: "b".into(),
                    agent_id: "sa-1".into(),
                    parent_id: None,
                    depth: 1,
                    tool_count: 2,
                    duration_secs: 8,
                    status: "completed".into(),
                },
            ],
            crate::spawn_history::TurnCommitMetrics::default(),
        );
        assert_eq!(turn.total_tools, 3);
        assert_eq!(turn.total_duration_secs, 8);
    }
}
