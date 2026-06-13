//! ASCII Gantt timeline for `/agents` — Hermes `GanttStrip` parity.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::agents_overlay::DelegateRow;
use crate::shelf_visual::fmt_duration;

#[derive(Clone, Debug)]
pub struct GanttSpan {
    pub task_index: usize,
    pub start_secs: u64,
    pub end_secs: u64,
}

pub fn build_gantt_spans(rows: &[DelegateRow]) -> Vec<GanttSpan> {
    if rows.is_empty() {
        return Vec::new();
    }
    let base = rows
        .iter()
        .map(|r| r.started_at)
        .min()
        .unwrap_or_else(std::time::Instant::now);
    rows.iter()
        .map(|row| {
            let start_secs = row
                .started_at
                .checked_duration_since(base)
                .unwrap_or(std::time::Duration::ZERO)
                .as_secs();
            let end_secs = start_secs + row.elapsed_secs.max(1);
            GanttSpan {
                task_index: row.task_index,
                start_secs,
                end_secs,
            }
        })
        .collect()
}

pub fn render_gantt_lines(
    spans: &[GanttSpan],
    cols: u16,
    cursor: usize,
    accent: Color,
    dim: Color,
    max_rows: usize,
) -> Vec<Line<'static>> {
    if spans.is_empty() || cols < 24 {
        return Vec::new();
    }

    let global_start = spans.iter().map(|s| s.start_secs).min().unwrap_or(0);
    let global_end = spans.iter().map(|s| s.end_secs).max().unwrap_or(1);
    let total = global_end.saturating_sub(global_start).max(1);
    let total_seconds = total;

    let id_gutter = 5usize;
    let label_reserve = 10usize;
    let bar_width = cols.saturating_sub(id_gutter as u16 + label_reserve as u16) as usize;
    let bar_width = bar_width.max(10);

    let start_idx = cursor
        .saturating_sub(max_rows / 2)
        .min(spans.len().saturating_sub(1));
    let shown = &spans[start_idx..spans.len().min(start_idx + max_rows)];

    let bar = |start: u64, end: u64| -> String {
        let s = ((start.saturating_sub(global_start) as f64 / total as f64) * bar_width as f64)
            .floor() as usize;
        let e = ((end.saturating_sub(global_start) as f64 / total as f64) * bar_width as f64)
            .ceil() as usize;
        let e = e.min(bar_width);
        let fill = e.saturating_sub(s).max(1);
        let bar = format!("{}{}", " ".repeat(s), "█".repeat(fill));
        format!("{bar}{}", " ".repeat(bar_width.saturating_sub(s + fill)))
    };

    let char_step = if total_seconds < 20 && bar_width > 20 {
        5
    } else {
        10
    };

    let mut ruler: Vec<char> = vec!['─'; bar_width];
    for (i, ch) in ruler.iter_mut().enumerate().skip(1) {
        if i % 10 == 0 {
            *ch = '┼';
        } else if i % 5 == 0 {
            *ch = '·';
        }
    }
    let ruler_str: String = ruler.into_iter().collect();

    let mut ruler_labels = vec![' '; bar_width];
    let mut pos = 0usize;
    while pos < bar_width {
        let secs = (pos as f64 / bar_width as f64) * total_seconds as f64;
        let label = if secs >= 60.0 {
            format!("{}m", (secs / 60.0) as u64)
        } else {
            format!("{}s", secs as u64)
        };
        for (i, ch) in label.chars().enumerate() {
            if pos + i < bar_width {
                ruler_labels[pos + i] = ch;
            }
        }
        pos += char_step;
    }
    let ruler_labels_str: String = ruler_labels.into_iter().collect();

    let mut lines = vec![
        Line::from(Span::styled(
            " timeline ",
            Style::default().fg(dim).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(" ".repeat(id_gutter), Style::default()),
            Span::styled(ruler_str, Style::default().fg(dim)),
        ]),
        Line::from(vec![
            Span::styled(" ".repeat(id_gutter), Style::default()),
            Span::styled(ruler_labels_str, Style::default().fg(dim)),
        ]),
    ];

    for span in shown {
        let id = format!("{:>3}", span.task_index + 1);
        let duration_label = fmt_duration(span.end_secs.saturating_sub(span.start_secs));
        lines.push(Line::from(vec![
            Span::styled(format!("{id} "), Style::default().fg(accent)),
            Span::styled(
                bar(span.start_secs, span.end_secs),
                Style::default().fg(accent),
            ),
            Span::styled(format!(" {duration_label}"), Style::default().fg(dim)),
        ]));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents_overlay::DelegateRow;
    use std::time::{Duration, Instant};

    fn row(task_index: usize, depth: u32, elapsed: u64, started: Instant) -> DelegateRow {
        DelegateRow {
            task_index,
            task_count: 2,
            goal: "g".into(),
            detail: None,
            agent_id: format!("sa-{task_index}"),
            parent_id: None,
            depth,
            tool_count: 0,
            current_tool: None,
            recent_tools: Vec::new(),
            elapsed_secs: elapsed,
            started_at: started,
        }
    }

    #[test]
    fn builds_relative_spans() {
        let base = Instant::now();
        let rows = vec![
            row(0, 1, 5, base),
            row(1, 1, 10, base + Duration::from_secs(2)),
        ];
        let spans = build_gantt_spans(&rows);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].start_secs, 0);
        assert_eq!(spans[1].start_secs, 2);
    }

    #[test]
    fn render_produces_timeline_header() {
        let base = Instant::now();
        let rows = vec![row(0, 1, 8, base), row(1, 1, 4, base)];
        let spans = build_gantt_spans(&rows);
        let lines = render_gantt_lines(&spans, 60, 0, Color::Yellow, Color::Gray, 4);
        assert!(lines.len() >= 4);
        assert!(lines[0].spans[0].content.contains("timeline"));
    }
}
