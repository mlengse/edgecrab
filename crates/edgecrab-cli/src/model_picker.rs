//! Model picker overlay stages — Hermes `modelPicker.tsx` disconnect parity.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::auth_cmd::{disconnect_catalog_provider, provider_disconnect_supported};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelSwitchIntent {
    Primary,
    Cheap,
    MoaAggregator,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ModelPickerStage {
    #[default]
    Browse,
    DisconnectConfirm {
        provider: String,
    },
    ExpensiveConfirm {
        model: String,
        message: String,
        intent: ModelSwitchIntent,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelPickerKeyAction {
    None,
    RequestDisconnect,
    ConfirmDisconnect,
    CancelDisconnect,
    ConfirmExpensive,
    CancelExpensive,
}

impl ModelPickerStage {
    pub fn reset(&mut self) {
        *self = Self::Browse;
    }
}

pub fn disconnect_help_suffix() -> &'static str {
    " · Ctrl+D disconnect"
}

pub fn browse_disconnect_shortcut(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('d') | KeyCode::Char('D'))
}

pub fn handle_picker_keys(stage: &ModelPickerStage, key: KeyEvent) -> ModelPickerKeyAction {
    match stage {
        ModelPickerStage::ExpensiveConfirm { .. } => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                ModelPickerKeyAction::ConfirmExpensive
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                ModelPickerKeyAction::CancelExpensive
            }
            _ => ModelPickerKeyAction::None,
        },
        ModelPickerStage::DisconnectConfirm { .. } => handle_disconnect_keys(stage, key),
        ModelPickerStage::Browse => {
            if browse_disconnect_shortcut(key) {
                ModelPickerKeyAction::RequestDisconnect
            } else {
                ModelPickerKeyAction::None
            }
        }
    }
}

pub fn handle_disconnect_keys(stage: &ModelPickerStage, key: KeyEvent) -> ModelPickerKeyAction {
    match stage {
        ModelPickerStage::DisconnectConfirm { .. } => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                ModelPickerKeyAction::ConfirmDisconnect
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                ModelPickerKeyAction::CancelDisconnect
            }
            _ => ModelPickerKeyAction::None,
        },
        _ => ModelPickerKeyAction::None,
    }
}

pub fn browse_disconnect_provider(selected_provider: Option<&str>) -> Option<String> {
    let provider = selected_provider?;
    if provider_disconnect_supported(provider) {
        Some(provider.to_string())
    } else {
        None
    }
}

pub fn render_expensive_confirm(
    frame: &mut Frame,
    area: Rect,
    model: &str,
    message: &str,
    theme: &Theme,
) {
    frame.render_widget(Clear, area);
    let popup = crate::overlay_layout::popup_rect(area, 72, 18.min(area.height));
    let accent = theme.shelf_accent.fg.unwrap_or(Color::Cyan);
    let warn = theme.output_error.fg.unwrap_or(Color::Rgb(239, 83, 80));
    let dim = theme.shelf_dim.fg.unwrap_or(Color::DarkGray);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(popup);

    let header = Paragraph::new(Line::from(Span::styled(
        " expensive model ",
        Style::default().fg(warn).add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(warn)),
    );
    frame.render_widget(header, chunks[0]);

    let mut body_lines: Vec<Line> = message
        .lines()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(dim))))
        .collect();
    if body_lines.is_empty() {
        body_lines.push(Line::from(Span::styled(
            format!("Switch to {model}?"),
            Style::default().fg(dim),
        )));
    }
    frame.render_widget(Paragraph::new(body_lines), chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Y ", Style::default().fg(accent)),
        Span::styled("confirm  ", Style::default().fg(dim)),
        Span::styled("N ", Style::default().fg(accent)),
        Span::styled("cancel  ", Style::default().fg(dim)),
        Span::styled("Esc ", Style::default().fg(accent)),
        Span::styled("back", Style::default().fg(dim)),
    ]));
    frame.render_widget(help, chunks[2]);
}

pub fn render_disconnect_confirm(frame: &mut Frame, area: Rect, provider: &str, theme: &Theme) {
    frame.render_widget(Clear, area);
    let popup = crate::overlay_layout::popup_rect(area, 58, 11);
    let accent = theme.shelf_accent.fg.unwrap_or(Color::Cyan);
    let warn = theme.output_error.fg.unwrap_or(Color::Rgb(239, 83, 80));
    let dim = theme.shelf_dim.fg.unwrap_or(Color::DarkGray);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(popup);

    let header = Paragraph::new(Line::from(Span::styled(
        " disconnect provider ",
        Style::default().fg(accent).add_modifier(Modifier::BOLD),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(accent)),
    );
    frame.render_widget(header, chunks[0]);

    let body = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Remove credentials for ", Style::default().fg(dim)),
            Span::styled(
                provider,
                Style::default().fg(warn).add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(dim)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Clears ~/.edgecrab/auth.json and ~/.edgecrab/.env keys for this provider.",
            Style::default().fg(dim).add_modifier(Modifier::ITALIC),
        )),
    ]);
    frame.render_widget(body, chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Y ", Style::default().fg(accent)),
        Span::styled("confirm  ", Style::default().fg(dim)),
        Span::styled("N ", Style::default().fg(accent)),
        Span::styled("cancel  ", Style::default().fg(dim)),
        Span::styled("Esc ", Style::default().fg(accent)),
        Span::styled("back", Style::default().fg(dim)),
    ]));
    frame.render_widget(help, chunks[2]);
}

pub fn execute_disconnect(provider: &str) -> Result<String, String> {
    disconnect_catalog_provider(provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn expensive_confirm_on_y() {
        let stage = ModelPickerStage::ExpensiveConfirm {
            model: "openai/gpt-5".into(),
            message: "expensive".into(),
            intent: ModelSwitchIntent::Primary,
        };
        assert_eq!(
            handle_picker_keys(&stage, key(KeyCode::Char('y'), KeyModifiers::NONE)),
            ModelPickerKeyAction::ConfirmExpensive
        );
    }

    #[test]
    fn expensive_cancel_on_n() {
        let stage = ModelPickerStage::ExpensiveConfirm {
            model: "openai/gpt-5".into(),
            message: "expensive".into(),
            intent: ModelSwitchIntent::Primary,
        };
        assert_eq!(
            handle_picker_keys(&stage, key(KeyCode::Char('n'), KeyModifiers::NONE)),
            ModelPickerKeyAction::CancelExpensive
        );
    }

    #[test]
    fn confirm_on_y_in_disconnect_stage() {
        let stage = ModelPickerStage::DisconnectConfirm {
            provider: "openai".into(),
        };
        assert_eq!(
            handle_disconnect_keys(&stage, key(KeyCode::Char('y'), KeyModifiers::NONE)),
            ModelPickerKeyAction::ConfirmDisconnect
        );
    }

    #[test]
    fn cancel_on_esc() {
        let stage = ModelPickerStage::DisconnectConfirm {
            provider: "openai".into(),
        };
        assert_eq!(
            handle_disconnect_keys(&stage, key(KeyCode::Esc, KeyModifiers::NONE)),
            ModelPickerKeyAction::CancelDisconnect
        );
    }

    #[test]
    fn browse_open_requires_supported_provider() {
        assert!(browse_disconnect_provider(Some("openai")).is_some());
        assert!(browse_disconnect_provider(Some("unknown-vendor-xyz")).is_none());
    }
}
