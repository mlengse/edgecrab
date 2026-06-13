//! Secret / sudo masked-input overlay (Hermes `MaskedPrompt` parity).

use super::*;

impl App {
    /// Handle a key press when the secret-capture overlay is active.
    pub(super) fn handle_secret_capture_key(&mut self, key: crossterm::event::KeyEvent) {
        match crate::secret_capture_overlay::map_overlay_text_input_key(key.code, key.modifiers) {
            crate::overlay_text_input::OverlayTextInputAction::AppendChar(c) => {
                if let DisplayState::SecretCapture { ref mut buffer, .. } = self.display_state {
                    buffer.push(c);
                }
            }
            crate::overlay_text_input::OverlayTextInputAction::Backspace => {
                if let DisplayState::SecretCapture { ref mut buffer, .. } = self.display_state {
                    buffer.pop();
                }
            }
            crate::overlay_text_input::OverlayTextInputAction::Submit => {
                let secret = if let DisplayState::SecretCapture { ref mut buffer, .. } =
                    self.display_state
                {
                    let s = buffer.clone();
                    buffer.clear();
                    s
                } else {
                    String::new()
                };
                if let Some(tx) = self.secret_pending_tx.take() {
                    let _ = tx.send(secret);
                }
                self.display_state = DisplayState::AwaitingFirstToken {
                    frame: 0,
                    started: std::time::Instant::now(),
                };
            }
            crate::overlay_text_input::OverlayTextInputAction::Cancel => {
                if let DisplayState::SecretCapture { ref mut buffer, .. } = self.display_state {
                    buffer.clear();
                }
                if let Some(tx) = self.secret_pending_tx.take() {
                    let _ = tx.send(String::new());
                }
                self.display_state = DisplayState::Idle;
            }
            crate::overlay_text_input::OverlayTextInputAction::Noop => {}
        }
        self.needs_redraw = true;
    }

    /// Render a masked-input overlay for secret/sudo capture.
    pub(super) fn render_secret_capture_overlay(&self, frame: &mut Frame, area: Rect) {
        let (var_name, prompt, is_sudo, buffer_len) = if let DisplayState::SecretCapture {
            ref var_name,
            ref prompt,
            is_sudo,
            ref buffer,
        } = self.display_state
        {
            (
                var_name.as_str(),
                prompt.as_str(),
                is_sudo,
                buffer.chars().count(),
            )
        } else {
            return;
        };

        frame.render_widget(Clear, area);

        let dlg_w = area.width.min(60);
        let dlg_h = 8u16;
        let x = area.x + (area.width.saturating_sub(dlg_w)) / 2;
        let y = area.y + (area.height.saturating_sub(dlg_h)) / 2;
        let dlg = Rect::new(x, y, dlg_w, dlg_h);

        let accent = if is_sudo {
            Color::Rgb(220, 80, 80)
        } else {
            Color::Rgb(80, 180, 220)
        };
        let icon = crate::secret_capture_overlay::secret_prompt_icon(is_sudo);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(dlg);

        let prompt_para = Paragraph::new(Line::from(vec![
            Span::styled(format!("  {icon} "), Style::default().fg(accent)),
            Span::styled(
                prompt,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                .border_style(Style::default().fg(accent))
                .title(format!(" {} ", var_name)),
        );
        frame.render_widget(prompt_para, chunks[0]);

        let masked = crate::secret_capture_overlay::secret_masked_display(buffer_len);
        let input_para = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(masked, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(accent)),
        ]))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::BOTTOM | Borders::RIGHT)
                .border_style(Style::default().fg(accent)),
        );
        frame.render_widget(input_para, chunks[1]);

        let help = Paragraph::new(Line::from(vec![
            Span::styled("  Enter ", Style::default().fg(accent)),
            Span::styled("submit  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(accent)),
            Span::styled("abort", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(help, chunks[2]);
    }
}
