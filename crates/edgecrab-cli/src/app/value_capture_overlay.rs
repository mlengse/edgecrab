//! Inline value-capture overlay — gateway bind, profiles, config fields.

use super::*;

impl App {
    pub(super) fn open_value_capture(
        &mut self,
        title: impl Into<String>,
        prompt: impl Into<String>,
        placeholder: impl Into<String>,
        masked: bool,
        buffer: impl Into<String>,
        action: ValueCaptureAction,
    ) {
        self.display_state = DisplayState::ValueCapture {
            title: title.into(),
            prompt: prompt.into(),
            placeholder: placeholder.into(),
            masked,
            buffer: buffer.into(),
            action,
        };
        self.needs_redraw = true;
    }

    pub(super) fn handle_value_capture_key(&mut self, key: crossterm::event::KeyEvent) {
        match crate::value_capture_overlay::map_value_capture_key(key.code, key.modifiers) {
            crate::value_capture_overlay::ValueCaptureKeyAction::AppendChar(c) => {
                if let DisplayState::ValueCapture { ref mut buffer, .. } = self.display_state {
                    buffer.push(c);
                }
            }
            crate::value_capture_overlay::ValueCaptureKeyAction::Backspace => {
                if let DisplayState::ValueCapture { ref mut buffer, .. } = self.display_state {
                    buffer.pop();
                }
            }
            crate::value_capture_overlay::ValueCaptureKeyAction::Submit => {
                let (action, value) = match &mut self.display_state {
                    DisplayState::ValueCapture { action, buffer, .. } => {
                        (action.clone(), buffer.trim().to_string())
                    }
                    _ => return,
                };
                self.display_state = DisplayState::Idle;
                self.apply_value_capture_action(action, value);
            }
            crate::value_capture_overlay::ValueCaptureKeyAction::Cancel => {
                self.display_state = DisplayState::Idle;
                self.needs_redraw = true;
            }
            crate::value_capture_overlay::ValueCaptureKeyAction::Noop => {}
        }
        self.needs_redraw = true;
    }

    pub(super) fn apply_value_capture_action(&mut self, action: ValueCaptureAction, value: String) {
        let refresh_gateway = matches!(
            &action,
            ValueCaptureAction::BindAddress
                | ValueCaptureAction::HomeChannel(_)
                | ValueCaptureAction::AllowedUsers(_)
                | ValueCaptureAction::PrimaryField(_)
        );

        match action {
            ValueCaptureAction::BindAddress => {
                let trimmed = value.trim();
                let bind = if trimmed.eq_ignore_ascii_case("clear") || trimmed.is_empty() {
                    "127.0.0.1:8080".to_string()
                } else {
                    trimmed.to_string()
                };
                let Some((host, port)) = bind.rsplit_once(':') else {
                    self.push_output(
                        "Bind must use host:port format, for example 127.0.0.1:8080",
                        OutputRole::Error,
                    );
                    return;
                };
                let Ok(port) = port.parse::<u16>() else {
                    self.push_output("Gateway port must be a valid TCP port.", OutputRole::Error);
                    return;
                };
                if port == 0 {
                    self.push_output(
                        "Gateway port must be between 1 and 65535.",
                        OutputRole::Error,
                    );
                    return;
                }
                let mut config = self.load_runtime_config();
                config.gateway.host = host.trim().to_string();
                config.gateway.port = port;
                match config.save() {
                    Ok(()) => self.push_output(
                        format!(
                            "Gateway bind set to {}:{}",
                            config.gateway.host, config.gateway.port
                        ),
                        OutputRole::System,
                    ),
                    Err(error) => self.push_output(
                        format!("Failed to save gateway bind: {error}"),
                        OutputRole::Error,
                    ),
                }
            }
            ValueCaptureAction::HomeChannel(platform) => {
                let mut config = self.load_runtime_config();
                let channel = if value.is_empty() || value.eq_ignore_ascii_case("clear") {
                    None
                } else {
                    Some(value)
                };
                match self.set_home_channel_in_config(&mut config, &platform, channel.clone()) {
                    Ok(()) => match config.save() {
                        Ok(()) => self.push_output(
                            match channel {
                                Some(channel) => {
                                    format!("Home channel for {platform} set to: {channel}")
                                }
                                None => format!("Home channel for {platform} cleared."),
                            },
                            OutputRole::System,
                        ),
                        Err(error) => self.push_output(
                            format!("Failed to save {platform} home channel: {error}"),
                            OutputRole::Error,
                        ),
                    },
                    Err(error) => self.push_output(error.to_string(), OutputRole::Error),
                }
            }
            ValueCaptureAction::AllowedUsers(platform) => {
                let values = if value.is_empty() || value.eq_ignore_ascii_case("clear") {
                    Vec::new()
                } else {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|entry| !entry.is_empty())
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                };
                let mut config = self.load_runtime_config();
                let env_key = format!("{}_ALLOWED_USERS", platform.to_ascii_uppercase());
                let save_result: Result<(), String> = match platform.as_str() {
                    "telegram" => {
                        config.gateway.telegram.allowed_users = values.clone();
                        config.save().map_err(|error| error.to_string())
                    }
                    "discord" => {
                        config.gateway.discord.allowed_users = values.clone();
                        config.save().map_err(|error| error.to_string())
                    }
                    "slack" => {
                        config.gateway.slack.allowed_users = values.clone();
                        config.save().map_err(|error| error.to_string())
                    }
                    "signal" => {
                        config.gateway.signal.allowed_users = values.clone();
                        config.save().map_err(|error| error.to_string())
                    }
                    "whatsapp" => {
                        config.gateway.whatsapp.allowed_users = values.clone();
                        config.save().map_err(|error| error.to_string())
                    }
                    _ => if values.is_empty() {
                        crate::gateway_setup::remove_env_key(&env_key)
                    } else {
                        crate::gateway_setup::save_env_key(&env_key, &values.join(","))
                    }
                    .map_err(|error| error.to_string()),
                };
                match save_result {
                    Ok(()) => self.push_output(
                        if values.is_empty() {
                            format!("{platform} allowlist cleared.")
                        } else {
                            format!("{platform} allowlist updated ({} entrie(s)).", values.len())
                        },
                        OutputRole::System,
                    ),
                    Err(error) => self.push_output(
                        format!("Failed to save {platform} allowlist: {error}"),
                        OutputRole::Error,
                    ),
                }
            }
            ValueCaptureAction::PrimaryField(field) => {
                if !self.save_gateway_primary_field(&field, &value) {
                    return;
                }
            }
            ValueCaptureAction::ProfileCreate => {
                let tokens = match shell_words::split(&value) {
                    Ok(tokens) => tokens,
                    Err(err) => {
                        self.push_output(format!("profile create: {err}"), OutputRole::Error);
                        return;
                    }
                };
                let Some(name) = tokens.first() else {
                    self.push_output(
                        "profile create: enter a name, for example `research --clone-from work`.",
                        OutputRole::Error,
                    );
                    return;
                };
                let clone = tokens.iter().any(|token| token == "--clone");
                let clone_all = tokens.iter().any(|token| token == "--clone-all");
                let clone_from = tokens
                    .windows(2)
                    .find_map(|window| (window[0] == "--clone-from").then_some(window[1].as_str()));
                self.execute_profile_create(name, clone, clone_all, clone_from);
            }
            ValueCaptureAction::ProfileRename(old_name) => {
                if value.is_empty() {
                    self.push_output(
                        "profile rename: enter the new profile name.",
                        OutputRole::Error,
                    );
                    return;
                }
                self.execute_profile_rename(&old_name, &value);
            }
            ValueCaptureAction::ProfileDeleteConfirm(name) => {
                if value != name {
                    self.push_output(
                        format!("profile delete: type `{name}` exactly to confirm."),
                        OutputRole::Error,
                    );
                    return;
                }
                self.execute_profile_delete(&name);
            }
            ValueCaptureAction::ProfileAlias(name) => {
                if value.eq_ignore_ascii_case("clear") || value.is_empty() {
                    self.execute_profile_alias(&name, true, None);
                } else {
                    self.execute_profile_alias(&name, false, Some(value.as_str()));
                }
            }
            ValueCaptureAction::ProfileExport(name) => {
                let output = (!value.is_empty()).then_some(value.as_str());
                self.execute_profile_export(&name, output);
            }
            ValueCaptureAction::ProfileImport => {
                let tokens = match shell_words::split(&value) {
                    Ok(tokens) => tokens,
                    Err(err) => {
                        self.push_output(format!("profile import: {err}"), OutputRole::Error);
                        return;
                    }
                };
                let Some(archive) = tokens.first() else {
                    self.push_output(
                        "profile import: enter an archive path and optional target name.",
                        OutputRole::Error,
                    );
                    return;
                };
                self.execute_profile_import(archive, tokens.get(1).map(String::as_str));
            }
        }
        if refresh_gateway {
            self.refresh_gateway_browser();
        }
    }

    pub(super) fn render_value_capture_overlay(&self, frame: &mut Frame, area: Rect) {
        let (title, prompt, placeholder, masked, buffer) = if let DisplayState::ValueCapture {
            ref title,
            ref prompt,
            ref placeholder,
            masked,
            ref buffer,
            ..
        } = self.display_state
        {
            (
                title.as_str(),
                prompt.as_str(),
                placeholder.as_str(),
                masked,
                buffer.as_str(),
            )
        } else {
            return;
        };

        frame.render_widget(Clear, area);

        let dlg_w = area.width.min(84);
        let dlg_h = 8u16;
        let x = area.x + (area.width.saturating_sub(dlg_w)) / 2;
        let y = area.y + (area.height.saturating_sub(dlg_h)) / 2;
        let dlg = Rect::new(x, y, dlg_w, dlg_h);
        let accent = Color::Rgb(120, 220, 200);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(dlg);

        let prompt_para = Paragraph::new(Line::from(vec![
            Span::styled("  ⛵ ", Style::default().fg(accent)),
            Span::styled(
                prompt,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::TOP | Borders::RIGHT)
                .border_style(Style::default().fg(accent))
                .title(format!(" {} ", title)),
        );
        frame.render_widget(prompt_para, chunks[0]);

        let visible =
            crate::value_capture_overlay::value_capture_visible_text(buffer, placeholder, masked);
        let input_style = if crate::value_capture_overlay::value_capture_uses_placeholder(buffer) {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        let input_para = Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(visible, input_style),
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
            Span::styled("save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(accent)),
            Span::styled("cancel", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(help, chunks[2]);
    }
}
