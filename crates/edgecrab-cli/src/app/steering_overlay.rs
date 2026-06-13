//! Mission steering overlay — compact Ctrl+S panel for HINT / REDIRECT / STOP steers.

use super::*;

impl App {
    /// Open the compact steering overlay, resetting its input and kind.
    pub(super) fn open_steering_overlay(&mut self) {
        let lines: Vec<String> = self
            .steering_textarea
            .lines()
            .iter()
            .map(|l| l.to_string())
            .collect();
        for _ in 0..lines.len() {
            self.steering_textarea
                .move_cursor(tui_textarea::CursorMove::End);
            self.steering_textarea.delete_line_by_head();
        }
        self.steering_overlay_active = true;
        self.needs_redraw = true;
    }

    /// Handle a key event while the steering overlay is open.
    pub(super) fn handle_steering_overlay_key(&mut self, key: event::KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.steering_overlay_active = false;
                self.needs_redraw = true;
            }
            (_, KeyCode::Tab) => {
                use edgecrab_core::SteeringKind;
                self.steering_kind = match self.steering_kind {
                    SteeringKind::Hint => SteeringKind::Redirect,
                    SteeringKind::Redirect => SteeringKind::Stop,
                    SteeringKind::Stop => SteeringKind::Hint,
                };
                self.needs_redraw = true;
            }
            (_, KeyCode::Enter) => {
                self.send_steer_from_overlay();
            }
            _ => {
                self.steering_textarea.input(key);
                self.needs_redraw = true;
            }
        }
    }

    /// Send the steer composed in the overlay, then close it.
    fn send_steer_from_overlay(&mut self) {
        let text: String = self.steering_textarea.lines().join("\n");
        let text = text.trim().to_string();
        self.steering_overlay_active = false;
        self.needs_redraw = true;

        if text.is_empty() {
            return;
        }

        let kind = self.steering_kind.clone();

        if !self.is_processing {
            let promoted = format!(
                "[⛵ STEER/{kind}] {text}",
                kind = match &kind {
                    edgecrab_core::SteeringKind::Hint => "HINT",
                    edgecrab_core::SteeringKind::Redirect => "REDIRECT",
                    edgecrab_core::SteeringKind::Stop => "STOP",
                },
            );
            self.push_output(
                "⛵ Steering → sent as new message (agent idle)".to_string(),
                OutputRole::System,
            );
            self.process_input(&promoted);
            return;
        }

        let send_result = if let Some(agent) = self.agent.as_ref() {
            Some(agent.send_steering(edgecrab_core::SteeringEvent::new(
                kind.clone(),
                text.clone(),
            )))
        } else {
            self.steer_tx.as_ref().map(|tx| {
                tx.send(edgecrab_core::SteeringEvent::new(
                    kind.clone(),
                    text.clone(),
                ))
            })
        };

        match send_result {
            Some(Ok(())) => {
                self.push_output(
                    format!(
                        "⛵ Steer queued ({}/{})",
                        match &kind {
                            edgecrab_core::SteeringKind::Hint => "HINT",
                            edgecrab_core::SteeringKind::Redirect => "REDIRECT",
                            edgecrab_core::SteeringKind::Stop => "STOP",
                        },
                        edgecrab_core::safe_truncate(&text, 60),
                    ),
                    OutputRole::System,
                );
                self.needs_redraw = true;
            }
            Some(Err(_)) => {
                self.steer_tx = None;
                self.push_output(
                    "⚠ Steer channel closed (new session?). Steering as new message.".to_string(),
                    OutputRole::Error,
                );
                let promoted = format!("[⛵ STEER] {text}");
                self.process_input(&promoted);
            }
            None => {
                self.push_output(
                    "⚠ No active agent for steering. Send as new message instead.".to_string(),
                    OutputRole::Error,
                );
            }
        }
    }

    /// Render the compact mission-steering overlay.
    pub(super) fn render_steering_overlay(&mut self, frame: &mut Frame, area: Rect) {
        use edgecrab_core::SteeringKind;

        let pw: u16 = 60.min(area.width);
        let ph: u16 = 7;
        let popup = Rect {
            x: area.x + area.width.saturating_sub(pw) / 2,
            y: area.y + (area.height.saturating_sub(ph) * 7 / 10),
            width: pw,
            height: ph,
        };
        frame.render_widget(Clear, popup);

        let border_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(120, 180, 255)))
            .title(Span::styled(
                " ⛵ Mission Steer ",
                Style::default()
                    .fg(Color::Rgb(200, 230, 255))
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = border_block.inner(popup);
        frame.render_widget(border_block, popup);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let (hint_s, redir_s, stop_s) = match self.steering_kind {
            SteeringKind::Hint => (
                Style::default()
                    .fg(Color::Rgb(10, 18, 30))
                    .bg(Color::Rgb(120, 220, 165))
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Rgb(90, 110, 140)),
                Style::default().fg(Color::Rgb(90, 110, 140)),
            ),
            SteeringKind::Redirect => (
                Style::default().fg(Color::Rgb(90, 110, 140)),
                Style::default()
                    .fg(Color::Rgb(10, 18, 30))
                    .bg(Color::Rgb(255, 200, 80))
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Rgb(90, 110, 140)),
            ),
            SteeringKind::Stop => (
                Style::default().fg(Color::Rgb(90, 110, 140)),
                Style::default().fg(Color::Rgb(90, 110, 140)),
                Style::default()
                    .fg(Color::Rgb(10, 18, 30))
                    .bg(Color::Rgb(255, 100, 80))
                    .add_modifier(Modifier::BOLD),
            ),
        };
        let kind_line = Line::from(vec![
            Span::styled("  kind: ", Style::default().fg(Color::Rgb(130, 150, 180))),
            Span::styled(" HINT ", hint_s),
            Span::raw(" "),
            Span::styled(" REDIRECT ", redir_s),
            Span::raw(" "),
            Span::styled(" STOP ", stop_s),
            Span::styled(
                "  (Tab=cycle)",
                Style::default().fg(Color::Rgb(70, 85, 110)),
            ),
        ]);
        frame.render_widget(Paragraph::new(kind_line), sections[0]);

        self.steering_textarea.set_style(
            Style::default()
                .fg(Color::Rgb(220, 235, 255))
                .bg(Color::Rgb(16, 20, 30)),
        );
        frame.render_widget(&self.steering_textarea, sections[1]);

        let help = Line::from(vec![
            Span::styled(
                " Enter",
                Style::default()
                    .fg(Color::Rgb(120, 220, 165))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" send  ", Style::default().fg(Color::Rgb(100, 130, 160))),
            Span::styled(
                " Tab",
                Style::default()
                    .fg(Color::Rgb(120, 220, 165))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" kind  ", Style::default().fg(Color::Rgb(100, 130, 160))),
            Span::styled(
                " Esc",
                Style::default()
                    .fg(Color::Rgb(200, 100, 80))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" cancel ", Style::default().fg(Color::Rgb(100, 130, 160))),
        ]);
        frame.render_widget(Paragraph::new(help), sections[2]);
    }
}
