//! `/tail` overlay — read-only view of a background process buffer (Hermes `process.list` parity).

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::overlay_layout::popup_rect;

pub const TAIL_PANEL_MAX_CHARS: usize = 4096;

#[derive(Clone, Debug, Default)]
pub struct ProcessTailPanel {
    pub active: bool,
    pub process_id: String,
    pub body: String,
    pub status_line: String,
    pub scroll_offset: u16,
}

impl ProcessTailPanel {
    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_content(
        &mut self,
        process_id: String,
        body: String,
        status: &str,
        exit_code: Option<i32>,
    ) {
        let trimmed = truncate_tail_body(&body, TAIL_PANEL_MAX_CHARS);
        let exit = exit_code
            .map(|c| format!(" exit {c}"))
            .unwrap_or_default();
        self.active = true;
        self.process_id = process_id;
        self.body = trimmed;
        self.status_line = format!("{status}{exit}");
        self.scroll_offset = 0;
    }
}

pub fn truncate_tail_body(body: &str, max_chars: usize) -> String {
    let trimmed = body.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let start = trimmed
        .char_indices()
        .nth(trimmed.chars().count().saturating_sub(max_chars))
        .map(|(i, _)| i)
        .unwrap_or(0);
    format!("…{}", &trimmed[start..])
}

/// Render the `/tail` popup overlay (Hermes `process.list` tail view parity).
pub fn render_process_tail_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &ProcessTailPanel,
    accent: Color,
    dim: Color,
) {
    frame.render_widget(Clear, area);
    let pw = (area.width * 9 / 10).max(20);
    let ph = (area.height * 4 / 5).max(8);
    let popup = popup_rect(area, pw, ph);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .title(format!(
            " tail: {} — {} ",
            panel.process_id, panel.status_line
        ));
    let lines: Vec<&str> = panel.body.lines().collect();
    let scroll = panel.scroll_offset as usize;
    let visible_height = chunks[0].height as usize;
    let visible: String = lines
        .iter()
        .skip(scroll)
        .take(visible_height)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    let paragraph = Paragraph::new(visible)
        .block(block)
        .style(Style::default().fg(dim))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, chunks[0]);
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(accent)),
        Span::styled("scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc ", Style::default().fg(accent)),
        Span::styled("close", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(help, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_tail() {
        let body = "x".repeat(5000);
        let out = truncate_tail_body(&body, 100);
        assert!(out.starts_with('…'));
        assert!(out.chars().count() <= 101);
    }
}
