//! Interactive `/details` picker — matches `/reasoning`, `/statusbar`, `/verbose` overlays.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::shelf_details::{ShelfDetailsMode, ShelfDetailsState, ShelfSection};
use crate::theme::Theme;

pub const ROW_COUNT: usize = 8;

/// Rows in the picker list (global block + per-section overrides).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailsPanelRow {
    GlobalHidden = 0,
    GlobalCollapsed = 1,
    GlobalExpanded = 2,
    GlobalCycle = 3,
    SectionThinking = 4,
    SectionTools = 5,
    SectionSubagents = 6,
    SectionActivity = 7,
}

impl DetailsPanelRow {
    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::GlobalHidden),
            1 => Some(Self::GlobalCollapsed),
            2 => Some(Self::GlobalExpanded),
            3 => Some(Self::GlobalCycle),
            4 => Some(Self::SectionThinking),
            5 => Some(Self::SectionTools),
            6 => Some(Self::SectionSubagents),
            7 => Some(Self::SectionActivity),
            _ => None,
        }
    }

    fn section(self) -> Option<ShelfSection> {
        match self {
            Self::SectionThinking => Some(ShelfSection::Thinking),
            Self::SectionTools => Some(ShelfSection::Tools),
            Self::SectionSubagents => Some(ShelfSection::Subagents),
            Self::SectionActivity => Some(ShelfSection::Activity),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DetailsPanel {
    pub active: bool,
    pub cursor: usize,
    pub dirty: bool,
}

impl DetailsPanel {
    pub fn open(&mut self, state: &ShelfDetailsState) {
        self.active = true;
        self.cursor = state.panel_cursor_for_global();
        self.dirty = false;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.dirty = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

pub enum DetailsPanelAction {
    None,
    Close,
    Changed,
}

pub fn handle_details_panel_key(
    panel: &mut DetailsPanel,
    key: KeyEvent,
    state: &mut ShelfDetailsState,
) -> DetailsPanelAction {
    if !panel.active {
        return DetailsPanelAction::None;
    }

    match key.code {
        KeyCode::Esc => {
            panel.close();
            DetailsPanelAction::Close
        }
        KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
            panel.cursor = panel.cursor.checked_sub(1).unwrap_or(ROW_COUNT - 1);
            DetailsPanelAction::None
        }
        KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
            panel.cursor = (panel.cursor + 1) % ROW_COUNT;
            DetailsPanelAction::None
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if let Some(row) = DetailsPanelRow::from_index(panel.cursor)
                && let Some(section) = row.section()
            {
                state.reset_section(section);
                panel.dirty = true;
                return DetailsPanelAction::Changed;
            }
            DetailsPanelAction::None
        }
        KeyCode::Char(c @ ('h' | 'H' | 'c' | 'C' | 'e' | 'E')) => {
            apply_mode_char(panel, state, c);
            DetailsPanelAction::Changed
        }
        KeyCode::Enter => {
            apply_row(panel, state, panel.cursor);
            DetailsPanelAction::Changed
        }
        _ => DetailsPanelAction::None,
    }
}

fn apply_mode_char(panel: &mut DetailsPanel, state: &mut ShelfDetailsState, c: char) {
    let mode = match c.to_ascii_lowercase() {
        'h' => ShelfDetailsMode::Hidden,
        'c' => ShelfDetailsMode::Collapsed,
        'e' => ShelfDetailsMode::Expanded,
        _ => return,
    };
    let Some(row) = DetailsPanelRow::from_index(panel.cursor) else {
        return;
    };
    match row.section() {
        None => state.set_global_mode(mode),
        Some(section) => state.set_section_mode(section, mode),
    }
    panel.dirty = true;
}

fn apply_row(panel: &mut DetailsPanel, state: &mut ShelfDetailsState, cursor: usize) {
    let Some(row) = DetailsPanelRow::from_index(cursor) else {
        return;
    };
    match row {
        DetailsPanelRow::GlobalHidden => state.set_global_mode(ShelfDetailsMode::Hidden),
        DetailsPanelRow::GlobalCollapsed => state.set_global_mode(ShelfDetailsMode::Collapsed),
        DetailsPanelRow::GlobalExpanded => state.set_global_mode(ShelfDetailsMode::Expanded),
        DetailsPanelRow::GlobalCycle => state.cycle_global_mode(),
        _ => {
            if let Some(section) = row.section() {
                state.cycle_section_mode(section);
            }
        }
    }
    panel.dirty = true;
}

pub fn render_details_panel(
    frame: &mut Frame,
    area: Rect,
    panel: &DetailsPanel,
    state: &ShelfDetailsState,
    theme: &Theme,
) {
    if !panel.active {
        return;
    }

    let accent = theme.shelf_accent.fg.unwrap_or(Color::Rgb(205, 175, 50));
    let dim = theme.shelf_dim.fg.unwrap_or(Color::Rgb(140, 140, 150));
    let border = theme.shelf_border;
    let warn = theme.shelf_hint.fg.unwrap_or(Color::Rgb(255, 200, 80));
    let heading = Color::Rgb(220, 210, 180);

    let popup = popup_rect(area, 78, 24);
    frame.render_widget(Clear, popup);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(popup);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(chunks[1]);

    let override_tag = if state.command_override {
        "override"
    } else {
        "default mix"
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled("  ◈  ", Style::default().fg(accent)),
        Span::styled(
            "Activity Shelf Disclosure",
            Style::default().fg(heading).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("global: {} · {}", state.global.label(), override_tag),
            Style::default().fg(dim),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .title(" /details "),
    );
    frame.render_widget(header, chunks[0]);

    let items = build_list_items(panel.cursor, state, accent, dim, warn);
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .title(Span::styled(" modes ", Style::default().fg(dim))),
    );
    frame.render_widget(list, cols[0]);

    let row = DetailsPanelRow::from_index(panel.cursor).unwrap_or(DetailsPanelRow::GlobalCollapsed);
    let detail_lines = detail_lines_for_row(row, state, accent, dim, warn);
    let detail = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .title(Span::styled(" preview ", Style::default().fg(dim))),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(detail, cols[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().fg(accent)),
        Span::styled("browse  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter ", Style::default().fg(accent)),
        Span::styled("apply/cycle  ", Style::default().fg(Color::DarkGray)),
        Span::styled("h/c/e ", Style::default().fg(accent)),
        Span::styled("set mode  ", Style::default().fg(Color::DarkGray)),
        Span::styled("r ", Style::default().fg(accent)),
        Span::styled("reset section  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc ", Style::default().fg(accent)),
        Span::styled("close", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(help, chunks[2]);
}

fn build_list_items(
    cursor: usize,
    state: &ShelfDetailsState,
    accent: Color,
    dim: Color,
    warn: Color,
) -> Vec<ListItem<'static>> {
    let mut items = Vec::with_capacity(ROW_COUNT);

    items.push(list_row(
        cursor == 0,
        accent,
        dim,
        "Global",
        "hidden",
        state.global == ShelfDetailsMode::Hidden && state.command_override,
        None,
    ));
    items.push(list_row(
        cursor == 1,
        accent,
        dim,
        "Global",
        "collapsed",
        state.global == ShelfDetailsMode::Collapsed && state.command_override,
        None,
    ));
    items.push(list_row(
        cursor == 2,
        accent,
        dim,
        "Global",
        "expanded",
        state.global == ShelfDetailsMode::Expanded && state.command_override,
        None,
    ));
    items.push(list_row(
        cursor == 3,
        accent,
        dim,
        "Global",
        "cycle all",
        false,
        Some((warn, "cycle")),
    ));

    for (i, section) in ShelfSection::ALL.iter().enumerate() {
        let row_idx = 4 + i;
        let eff = state.effective_mode(*section);
        let tag = if state.has_section_override(*section) {
            "override"
        } else if state.command_override {
            "global"
        } else {
            "default"
        };
        items.push(list_row(
            cursor == row_idx,
            accent,
            dim,
            section.name(),
            eff.label(),
            state.has_section_override(*section),
            Some((dim, tag)),
        ));
    }

    items
}

fn list_row(
    selected: bool,
    accent: Color,
    dim: Color,
    label: &str,
    value: &str,
    check: bool,
    tag: Option<(Color, &str)>,
) -> ListItem<'static> {
    let bg = if selected {
        Color::Rgb(35, 32, 24)
    } else {
        Color::Reset
    };
    let fg = if selected { Color::White } else { dim };
    let marker = if selected { "▸" } else { " " };
    let check_mark = if check { " ✓" } else { "" };
    let label_owned = format!("{label:<10}");
    let value_owned = value.to_string();
    let check_owned = check_mark.to_string();
    ListItem::new(Line::from({
        let mut spans = vec![
            Span::styled(format!(" {marker} "), Style::default().fg(accent).bg(bg)),
            Span::styled(label_owned, Style::default().fg(fg).bg(bg)),
            Span::styled(value_owned, Style::default().fg(accent).bg(bg)),
            Span::styled(
                check_owned,
                Style::default().fg(Color::Rgb(100, 200, 100)).bg(bg),
            ),
        ];
        if let Some((color, text)) = tag {
            spans.push(Span::styled(
                format!(" · {text}"),
                Style::default().fg(color).bg(bg),
            ));
        }
        spans
    }))
}

fn popup_rect(area: Rect, w: u16, h: u16) -> Rect {
    let pw = w.min(area.width);
    let ph = h.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(pw) / 2,
        y: area.y + area.height.saturating_sub(ph) / 2,
        width: pw,
        height: ph,
    }
}

fn detail_lines_for_row(
    row: DetailsPanelRow,
    state: &ShelfDetailsState,
    accent: Color,
    dim: Color,
    warn: Color,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match row {
        DetailsPanelRow::GlobalHidden
        | DetailsPanelRow::GlobalCollapsed
        | DetailsPanelRow::GlobalExpanded => {
            let mode = match row {
                DetailsPanelRow::GlobalHidden => ShelfDetailsMode::Hidden,
                DetailsPanelRow::GlobalExpanded => ShelfDetailsMode::Expanded,
                _ => ShelfDetailsMode::Collapsed,
            };
            lines.push(Line::from(vec![Span::styled(
                format!("Set all sections → {}", mode.label()),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(ShelfDetailsState::mode_blurb(mode)));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Clears per-section overrides.",
                Style::default().fg(dim),
            )]));
        }
        DetailsPanelRow::GlobalCycle => {
            lines.push(Line::from(vec![Span::styled(
                "Cycle global mode",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from("hidden → collapsed → expanded → hidden"));
            lines.push(Line::from(ShelfDetailsState::mode_blurb(
                state.global.cycle(),
            )));
        }
        _ => {
            if let Some(section) = row.section() {
                let eff = state.effective_mode(section);
                let def = state.default_mode(section);
                lines.push(Line::from(vec![Span::styled(
                    format!("Section: {}", section.name()),
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(ShelfDetailsState::section_blurb(section)));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Effective: ", Style::default().fg(dim)),
                    Span::styled(eff.label().to_string(), Style::default().fg(accent)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Default: ", Style::default().fg(dim)),
                    Span::styled(def.label().to_string(), Style::default().fg(dim)),
                ]));
                if state.has_section_override(section) {
                    lines.push(Line::from(vec![Span::styled(
                        "Per-section override active",
                        Style::default().fg(warn),
                    )]));
                } else if state.command_override {
                    lines.push(Line::from(vec![Span::styled(
                        "Following global override",
                        Style::default().fg(dim),
                    )]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Enter cycles mode · r resets to default",
                    Style::default().fg(dim),
                )]));
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn enter_on_global_collapsed_sets_mode() {
        let mut panel = DetailsPanel {
            active: true,
            cursor: 1,
            ..Default::default()
        };
        let mut state = ShelfDetailsState::default();
        let action = handle_details_panel_key(&mut panel, key(KeyCode::Enter), &mut state);
        assert!(matches!(action, DetailsPanelAction::Changed));
        assert_eq!(state.global, ShelfDetailsMode::Collapsed);
        assert!(state.command_override);
    }

    #[test]
    fn r_resets_section_override() {
        let mut panel = DetailsPanel {
            active: true,
            cursor: 4,
            ..Default::default()
        };
        let mut state = ShelfDetailsState::default();
        state.set_section_mode(ShelfSection::Thinking, ShelfDetailsMode::Hidden);
        handle_details_panel_key(&mut panel, key(KeyCode::Char('r')), &mut state);
        assert!(!state.has_section_override(ShelfSection::Thinking));
    }
}
