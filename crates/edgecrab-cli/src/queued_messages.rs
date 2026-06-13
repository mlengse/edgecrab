//! Queued follow-up prompts above the composer — Hermes `queuedMessages.tsx` parity.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Theme;
use edgecrab_core::safe_truncate;

/// Visible queue rows in the composer strip (Hermes `QUEUE_WINDOW`).
pub const QUEUE_WINDOW: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueueWindow {
    pub start: usize,
    pub end: usize,
    pub show_lead: bool,
    pub show_tail: bool,
}

/// Slice indices for the visible queue window.
pub fn get_queue_window(queue_len: usize, queue_edit_idx: Option<usize>) -> QueueWindow {
    if queue_len == 0 {
        return QueueWindow {
            start: 0,
            end: 0,
            show_lead: false,
            show_tail: false,
        };
    }
    let start = match queue_edit_idx {
        None => 0,
        Some(idx) => {
            let max_start = queue_len.saturating_sub(QUEUE_WINDOW);
            idx.saturating_sub(1).min(max_start)
        }
    };
    let end = (start + QUEUE_WINDOW).min(queue_len);
    QueueWindow {
        start,
        end,
        show_lead: start > 0,
        show_tail: end < queue_len,
    }
}

/// Total panel height: header + up to `QUEUE_WINDOW` rows + optional ellipsis rows.
pub fn panel_height(queue_len: usize) -> u16 {
    if queue_len == 0 {
        return 0;
    }
    let window = get_queue_window(queue_len, None);
    let mut lines = 1u16; // header
    if window.show_lead {
        lines += 1;
    }
    lines += (window.end - window.start) as u16;
    if window.show_tail {
        lines += 1;
    }
    lines
}

pub fn render_queued_messages(
    frame: &mut Frame,
    area: Rect,
    queued: &[String],
    queue_edit_idx: Option<usize>,
    theme: &Theme,
) {
    if queued.is_empty() || area.height == 0 {
        return;
    }

    let dim = theme.shelf_dim.fg.unwrap_or(Color::DarkGray);
    let accent = theme.shelf_accent.fg.unwrap_or(Color::Rgb(205, 175, 50));
    let window = get_queue_window(queued.len(), queue_edit_idx);
    let cols = area.width as usize;

    let header_suffix = queue_edit_idx.map_or(String::new(), |idx| {
        format!(" · editing {} · Ctrl+X delete · Esc cancel", idx + 1)
    });
    let mut lines = vec![Line::from(Span::styled(
        format!("queued ({}){header_suffix}", queued.len()),
        Style::default().fg(dim).add_modifier(Modifier::ITALIC),
    ))];

    if window.show_lead {
        lines.push(Line::from(Span::styled(" …", Style::default().fg(dim))));
    }

    for (offset, item) in queued[window.start..window.end].iter().enumerate() {
        let idx = window.start + offset;
        let active = queue_edit_idx == Some(idx);
        let marker = if active { "▸" } else { " " };
        let preview = safe_truncate(item.trim(), cols.saturating_sub(10).max(16));
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {marker} {}. ", idx + 1),
                Style::default().fg(if active { accent } else { dim }),
            ),
            Span::styled(
                preview,
                Style::default()
                    .fg(if active { accent } else { dim })
                    .add_modifier(if active {
                        Modifier::empty()
                    } else {
                        Modifier::DIM
                    }),
            ),
        ]));
    }

    if window.show_tail {
        lines.push(Line::from(Span::styled(
            format!("  …and {} more", queued.len() - window.end),
            Style::default().fg(dim),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_caps_at_three_rows() {
        let w = get_queue_window(7, None);
        assert_eq!(w.start, 0);
        assert_eq!(w.end, 3);
        assert!(!w.show_lead);
        assert!(w.show_tail);
    }

    #[test]
    fn window_follows_edit_index() {
        let w = get_queue_window(5, Some(3));
        assert_eq!(w.start, 2);
        assert_eq!(w.end, 5);
        assert!(w.show_lead);
        assert!(!w.show_tail);
    }

    #[test]
    fn panel_height_counts_ellipsis_rows() {
        assert_eq!(panel_height(0), 0);
        assert_eq!(panel_height(2), 3); // header + 2 rows
        assert_eq!(panel_height(7), 5); // header + … + 3 rows + …and N
    }
}
