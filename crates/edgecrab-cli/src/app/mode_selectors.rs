//! Display mode picker overlays (/verbose, /reasoning, /personality, …).

use super::*;

impl App {
    pub(super) fn render_verbose_selector(&self, frame: &mut Frame, area: Rect) {
        // Compact centered popup — 4 mode rows + header + detail + help.
        let popup = popup_rect(area, 72, 18);
        frame.render_widget(Clear, popup);
        let chunks = picker_three_layout(popup); // header(3) | body(min) | help(1)
        let body = picker_two_cols(chunks[1], 45); // list | detail

        // ── Modes metadata ───────────────────────────────────────
        const MODES: [(ToolProgressMode, &str, &str, &str); 4] = [
            (
                ToolProgressMode::Off,
                "OFF",
                "⊘",
                "Silent — only the status bar shows active work.",
            ),
            (
                ToolProgressMode::New,
                "NEW",
                "◑",
                "Show each distinct tool call once per turn.",
            ),
            (
                ToolProgressMode::All,
                "ALL",
                "●",
                "Show every tool call in the transcript.",
            ),
            (
                ToolProgressMode::Verbose,
                "VERBOSE",
                "◉",
                "Show every call + curated plan and result detail lines.",
            ),
        ];
        let active_mode = self.tool_progress_mode;
        let cursor = self.verbose_selector_cursor;

        // ── Header block ─────────────────────────────────────────
        let active_label = MODES
            .iter()
            .find(|(m, _, _, _)| *m == active_mode)
            .map(|(_, lbl, _, _)| *lbl)
            .unwrap_or("?");
        let header = Paragraph::new(Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(Color::Rgb(130, 210, 255))),
            Span::styled(
                "Tool Progress Display",
                Style::default()
                    .fg(Color::Rgb(200, 225, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("current: {active_label}"),
                Style::default().fg(Color::Rgb(100, 170, 220)),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(100, 160, 210)))
                .title(" /verbose "),
        );
        frame.render_widget(header, chunks[0]);

        // ── Mode list ────────────────────────────────────────────
        let list_items: Vec<ListItem> = MODES
            .iter()
            .enumerate()
            .map(|(i, (mode, label, icon, _desc))| {
                let is_cursor = i == cursor;
                let is_active = *mode == active_mode;
                let bg = if is_cursor {
                    Color::Rgb(22, 38, 55)
                } else {
                    Color::Rgb(15, 18, 24)
                };

                let marker = selector_marker(is_cursor, Color::Rgb(130, 210, 255), Some(bg));

                let active_badge = if is_active {
                    Span::styled(" ◉", Style::default().fg(Color::Rgb(100, 200, 130)))
                } else {
                    Span::styled(" ○", Style::default().fg(Color::Rgb(55, 65, 80)))
                };

                let icon_style = if is_cursor {
                    Style::default().bg(bg).fg(Color::Rgb(130, 210, 255))
                } else if is_active {
                    Style::default().fg(Color::Rgb(100, 200, 130))
                } else {
                    Style::default().fg(Color::Rgb(80, 95, 115))
                };
                let label_style = if is_cursor {
                    Style::default()
                        .bg(bg)
                        .fg(Color::Rgb(200, 225, 255))
                        .add_modifier(Modifier::BOLD)
                } else if is_active {
                    Style::default().fg(Color::Rgb(160, 210, 170))
                } else {
                    Style::default().fg(Color::Rgb(155, 165, 185))
                };

                ListItem::new(Line::from(vec![
                    marker,
                    active_badge,
                    Span::styled(format!("  {icon} "), icon_style),
                    Span::styled(unicode_pad_right(label, 9), label_style),
                ]))
            })
            .collect();

        let list = List::new(list_items).style(Style::default().bg(Color::Rgb(15, 18, 24)));
        frame.render_widget(list, body[0]);

        // ── Detail panel for highlighted mode ────────────────────
        let (_, detail_label, detail_icon, detail_desc) = &MODES[cursor];
        let is_active_cursor = MODES[cursor].0 == active_mode;
        let mut detail_lines: Vec<Line<'static>> = Vec::new();

        // Mode name heading
        detail_lines.push(Line::from(vec![
            Span::styled(
                format!("{detail_icon} {detail_label}"),
                Style::default()
                    .fg(Color::Rgb(130, 210, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            if is_active_cursor {
                Span::styled("  ◉ active", Style::default().fg(Color::Rgb(100, 200, 130)))
            } else {
                Span::raw("")
            },
        ]));
        detail_lines.push(Line::from(""));

        // Description — word-wrap at panel width
        detail_lines.push(Line::from(Span::styled(
            detail_desc.to_string(),
            Style::default().fg(Color::Rgb(185, 200, 220)),
        )));
        detail_lines.push(Line::from(""));

        // What pressing Enter will do
        if is_active_cursor {
            detail_lines.push(Line::from(Span::styled(
                "Already active — Enter closes.",
                Style::default().fg(Color::Rgb(110, 125, 140)),
            )));
        } else {
            detail_lines.push(Line::from(Span::styled(
                "Press Enter to switch to this mode.",
                Style::default().fg(Color::Rgb(130, 210, 255)),
            )));
        }

        let detail = Paragraph::new(Text::from(detail_lines))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(60, 85, 110)))
                    .title(" Details "),
            );
        frame.render_widget(detail, body[1]);

        // ── Help bar ─────────────────────────────────────────────
        frame.render_widget(
            Paragraph::new(picker_help_line(Color::Rgb(130, 210, 255))),
            chunks[2],
        );
    }

    /// Render the reasoning settings picker overlay.
    ///
    /// 5 options: Low / Medium / High effort (API) and Show / Hide reasoning trace.
    pub(super) fn render_reasoning_selector(&self, frame: &mut Frame, area: Rect) {
        let popup = popup_rect(area, 72, 20);
        frame.render_widget(Clear, popup);
        let chunks = picker_three_layout(popup);
        let body = picker_two_cols(chunks[1], 42);

        const ENTRIES: [(&str, &str, &str, &str); 5] = [
            ("low", "LOW", "◌", "Faster & cheaper. Less analytic depth."),
            ("medium", "MEDIUM", "◎", "Balanced — good for most tasks."),
            (
                "high",
                "HIGH",
                "●",
                "Deeper reasoning. Slower, more tokens.",
            ),
            (
                "show",
                "SHOW",
                "o",
                "Reveal the reasoning trace above answers.",
            ),
            ("hide", "HIDE", "x", "Keep the reasoning trace hidden."),
        ];

        let accent = Color::Rgb(130, 210, 255);
        let cursor = self.reasoning_selector_cursor;
        let cur_effort = self.reasoning_effort_hint.as_deref().unwrap_or("medium");
        let cur_vis = if self.show_reasoning {
            "shown"
        } else {
            "hidden"
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(accent)),
            Span::styled(
                "Reasoning Settings",
                Style::default()
                    .fg(Color::Rgb(200, 225, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("effort: {}  trace: {}", cur_effort.to_uppercase(), cur_vis),
                Style::default().fg(Color::Rgb(100, 170, 220)),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(100, 160, 210)))
                .title(" /reasoning "),
        );
        frame.render_widget(header, chunks[0]);

        // ── List panel ───────────────────────────────────────────
        let items: Vec<ListItem> = ENTRIES
            .iter()
            .enumerate()
            .map(|(i, (_, label, icon, _))| {
                let is_cursor = i == cursor;
                let bg = if is_cursor {
                    Color::Rgb(40, 60, 80)
                } else {
                    Color::Reset
                };
                let fg = if is_cursor {
                    Color::White
                } else {
                    Color::Rgb(180, 200, 220)
                };
                ListItem::new(Line::from(vec![
                    selector_marker(is_cursor, accent, Some(bg)),
                    Span::styled(format!(" {icon} "), Style::default().fg(accent).bg(bg)),
                    Span::styled(
                        format!("{label:<8}", label = *label),
                        Style::default().fg(fg).bg(bg),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items).block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(list, body[0]);

        // ── Detail panel ─────────────────────────────────────────
        let (key, label, icon, desc) = ENTRIES[cursor];
        let current_active = match cursor {
            0 => self.reasoning_effort_hint.as_deref() == Some("low"),
            1 => self.reasoning_effort_hint.as_deref().unwrap_or("medium") == "medium",
            2 => self.reasoning_effort_hint.as_deref() == Some("high"),
            3 => self.show_reasoning,
            4 => !self.show_reasoning,
            _ => false,
        };
        let _ = key; // used via cursor index
        let action_hint = if current_active {
            "Already active"
        } else {
            "Press Enter to apply"
        };
        let detail = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {icon} {label}"),
                Style::default()
                    .fg(Color::Rgb(200, 225, 255))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {desc}"),
                Style::default().fg(Color::Rgb(170, 195, 215)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {action_hint}"),
                Style::default().fg(if current_active {
                    Color::Rgb(100, 200, 100)
                } else {
                    Color::Rgb(220, 180, 80)
                }),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(60, 90, 120))),
        );
        frame.render_widget(detail, body[1]);

        // ── Help bar ─────────────────────────────────────────────
        frame.render_widget(Paragraph::new(picker_help_line(accent)), chunks[2]);
    }

    /// Render the personality picker overlay.
    ///
    /// Shows all personality presets with a preview pane. The first entry
    /// is always "clear" to remove any active personality overlay.
    pub(super) fn render_personality_selector(&self, frame: &mut Frame, area: Rect) {
        let popup = popup_rect(area, 76, 22);
        frame.render_widget(Clear, popup);
        let chunks = picker_three_layout(popup);
        let body = picker_two_cols(chunks[1], 38);

        let accent = Color::Rgb(200, 160, 255);
        let cursor = self.personality_selector_cursor;
        let entries = &self.personality_selector_entries;
        let cur_personality = self.session_personality.as_deref().unwrap_or("default");

        // ── Header ───────────────────────────────────────────────
        let header = Paragraph::new(Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(accent)),
            Span::styled(
                "Personality",
                Style::default()
                    .fg(Color::Rgb(225, 200, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("active: {cur_personality}"),
                Style::default().fg(Color::Rgb(170, 140, 220)),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(150, 100, 200)))
                .title(" /personality "),
        );
        frame.render_widget(header, chunks[0]);

        // ── Scrolling list panel ──────────────────────────────────
        let list_height = body[0].height.saturating_sub(2) as usize;
        let total = entries.len();
        let scroll_start = if total > list_height && cursor >= list_height {
            cursor.saturating_sub(list_height.saturating_sub(1))
        } else {
            0
        };
        let name_w = body[0].width.saturating_sub(6) as usize;
        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .skip(scroll_start)
            .take(list_height)
            .map(|(i, (name, _))| {
                let is_cursor = i == cursor;
                let is_active = name == "clear" && self.session_personality.is_none()
                    || self.session_personality.as_deref() == Some(name.as_str());
                let bg = if is_cursor {
                    Color::Rgb(60, 30, 80)
                } else {
                    Color::Reset
                };
                let fg = if is_cursor {
                    Color::White
                } else {
                    Color::Rgb(200, 170, 230)
                };
                let badge_fg = Color::Rgb(100, 200, 100);
                let name_cell = unicode_trunc(name, name_w.max(1));
                let badge = if is_active { " ✓" } else { "" };
                ListItem::new(Line::from(vec![
                    selector_marker(is_cursor, accent, Some(bg)),
                    Span::styled(format!("  {name_cell}"), Style::default().fg(fg).bg(bg)),
                    Span::styled(badge, Style::default().fg(badge_fg).bg(bg)),
                ]))
            })
            .collect();
        let list = List::new(items).block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(list, body[0]);

        // ── Detail panel ─────────────────────────────────────────
        if let Some((name, preview)) = entries.get(cursor) {
            let is_active = (name == "clear" && self.session_personality.is_none())
                || self.session_personality.as_deref() == Some(name.as_str());
            let action_hint = if is_active {
                "Already active"
            } else {
                "Press Enter to apply"
            };
            let preview_short = edgecrab_core::safe_truncate(preview, 240);
            let detail = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {name}"),
                    Style::default()
                        .fg(Color::Rgb(225, 200, 255))
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {preview_short}"),
                    Style::default().fg(Color::Rgb(195, 175, 215)),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {action_hint}"),
                    Style::default().fg(if is_active {
                        Color::Rgb(100, 200, 100)
                    } else {
                        Color::Rgb(220, 180, 80)
                    }),
                )),
            ])
            .wrap(ratatui::widgets::Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(90, 60, 120))),
            );
            frame.render_widget(detail, body[1]);
        }

        // ── Help bar ─────────────────────────────────────────────
        frame.render_widget(Paragraph::new(picker_help_line(accent)), chunks[2]);
    }

    /// Render the streaming mode picker (2 options: on / off).
    pub(super) fn render_stream_selector(&self, frame: &mut Frame, area: Rect) {
        let popup = popup_rect(area, 64, 16);
        frame.render_widget(Clear, popup);
        let chunks = picker_three_layout(popup);
        let body = picker_two_cols(chunks[1], 38);

        const ENTRIES: [(&str, &str, &str); 2] = [
            ("on", "ON", "Tokens stream live as they are generated."),
            ("off", "OFF", "Replies appear as one complete message."),
        ];

        let accent = Color::Rgb(255, 210, 80);
        let cursor = self.stream_selector_cursor;
        let cur_label = if self.streaming_enabled { "on" } else { "off" };

        let header = Paragraph::new(Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(accent)),
            Span::styled(
                "Token Streaming",
                Style::default()
                    .fg(Color::Rgb(255, 235, 160))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("current: {cur_label}"),
                Style::default().fg(Color::Rgb(200, 170, 80)),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(180, 150, 60)))
                .title(" /stream "),
        );
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = ENTRIES
            .iter()
            .enumerate()
            .map(|(i, (key, label, _))| {
                let is_cursor = i == cursor;
                let is_active = *key == cur_label;
                let bg = if is_cursor {
                    Color::Rgb(70, 55, 20)
                } else {
                    Color::Reset
                };
                let fg = if is_cursor {
                    Color::White
                } else {
                    Color::Rgb(230, 200, 140)
                };
                let badge_fg = Color::Rgb(100, 200, 100);
                ListItem::new(Line::from(vec![
                    selector_marker(is_cursor, accent, Some(bg)),
                    Span::styled(
                        format!("  {label:<5}", label = *label),
                        Style::default().fg(fg).bg(bg),
                    ),
                    Span::styled(
                        if is_active { " ✓" } else { "" },
                        Style::default().fg(badge_fg).bg(bg),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items).block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(list, body[0]);

        let (key, label, desc) = ENTRIES[cursor];
        let is_active_cur = key == cur_label;
        let action_hint = if is_active_cur {
            "Already active"
        } else {
            "Press Enter to switch"
        };
        let detail = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {label}"),
                Style::default()
                    .fg(Color::Rgb(255, 235, 160))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {desc}"),
                Style::default().fg(Color::Rgb(210, 185, 130)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {action_hint}"),
                Style::default().fg(if is_active_cur {
                    Color::Rgb(100, 200, 100)
                } else {
                    Color::Rgb(220, 180, 80)
                }),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(100, 80, 30))),
        );
        frame.render_widget(detail, body[1]);

        frame.render_widget(Paragraph::new(picker_help_line(accent)), chunks[2]);
    }

    /// Render the status bar visibility picker (2 options: visible / hidden).
    pub(super) fn render_statusbar_selector(&self, frame: &mut Frame, area: Rect) {
        let popup = popup_rect(area, 64, 16);
        frame.render_widget(Clear, popup);
        let chunks = picker_three_layout(popup);
        let body = picker_two_cols(chunks[1], 38);

        const ENTRIES: [(&str, &str, &str); 2] = [
            (
                "visible",
                "Visible",
                "Shows model, context pressure, and metrics.",
            ),
            (
                "hidden",
                "Hidden",
                "Clean mode — full height for conversation.",
            ),
        ];

        let accent = Color::Rgb(120, 200, 200);
        let cursor = self.statusbar_selector_cursor;
        let cur_label = if self.show_status_bar {
            "visible"
        } else {
            "hidden"
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(accent)),
            Span::styled(
                "Status Bar",
                Style::default()
                    .fg(Color::Rgb(180, 230, 230))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("current: {cur_label}"),
                Style::default().fg(Color::Rgb(100, 170, 170)),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(80, 160, 160)))
                .title(" /statusbar "),
        );
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = ENTRIES
            .iter()
            .enumerate()
            .map(|(i, (key, label, _))| {
                let is_cursor = i == cursor;
                let is_active = *key == cur_label;
                let bg = if is_cursor {
                    Color::Rgb(20, 60, 60)
                } else {
                    Color::Reset
                };
                let fg = if is_cursor {
                    Color::White
                } else {
                    Color::Rgb(160, 210, 210)
                };
                let badge_fg = Color::Rgb(100, 200, 100);
                ListItem::new(Line::from(vec![
                    selector_marker(is_cursor, accent, Some(bg)),
                    Span::styled(format!("  {label}"), Style::default().fg(fg).bg(bg)),
                    Span::styled(
                        if is_active { " ✓" } else { "" },
                        Style::default().fg(badge_fg).bg(bg),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items).block(Block::default().borders(Borders::LEFT | Borders::RIGHT));
        frame.render_widget(list, body[0]);

        let (key, label, desc) = ENTRIES[cursor];
        let is_active_cur = key == cur_label;
        let action_hint = if is_active_cur {
            "Already active"
        } else {
            "Press Enter to switch"
        };
        let detail = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {label}"),
                Style::default()
                    .fg(Color::Rgb(180, 230, 230))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {desc}"),
                Style::default().fg(Color::Rgb(150, 200, 200)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {action_hint}"),
                Style::default().fg(if is_active_cur {
                    Color::Rgb(100, 200, 100)
                } else {
                    Color::Rgb(220, 180, 80)
                }),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(50, 110, 110))),
        );
        frame.render_widget(detail, body[1]);

        frame.render_widget(Paragraph::new(picker_help_line(accent)), chunks[2]);
    }

    /// Render the shadow judge picker (2 options: on / off).
    pub(super) fn render_shadow_judge_selector(&self, frame: &mut Frame, area: Rect) {
        let popup = popup_rect(area, 74, 18);
        frame.render_widget(Clear, popup);
        let chunks = picker_three_layout(popup);
        let body = picker_two_cols(chunks[1], 42);

        const ENTRIES: [(&str, &str, &str); 2] = [
            (
                "on",
                "ON",
                "Run the completion oracle before finalizing; vetoes likely-incomplete stops.",
            ),
            (
                "off",
                "OFF",
                "Skip completion verification and trust the normal completion policy only.",
            ),
        ];

        let accent = Color::Rgb(130, 200, 255);
        let cursor = self.shadow_judge_selector_cursor;
        let cur_label = if self.shadow_judge_enabled {
            "on"
        } else {
            "off"
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled("  ◈  ", Style::default().fg(accent)),
            Span::styled(
                "Shadow Judge",
                Style::default()
                    .fg(Color::Rgb(210, 235, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("current: {}", cur_label.to_uppercase()),
                Style::default().fg(Color::Rgb(145, 190, 230)),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(90, 145, 195)))
                .title(" /shadow-judge "),
        );
        frame.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = ENTRIES
            .iter()
            .enumerate()
            .map(|(i, (key, label, _))| {
                let is_cursor = i == cursor;
                let is_active = *key == cur_label;
                let bg = if is_cursor {
                    Color::Rgb(28, 52, 74)
                } else {
                    Color::Reset
                };
                let fg = if is_cursor {
                    Color::White
                } else {
                    Color::Rgb(185, 215, 240)
                };
                ListItem::new(Line::from(vec![
                    selector_marker(is_cursor, accent, Some(bg)),
                    Span::styled(
                        format!("  {label:<4}", label = *label),
                        Style::default().fg(fg).bg(bg),
                    ),
                    Span::styled(
                        if is_active { " ✓" } else { "" },
                        Style::default().fg(Color::Rgb(105, 210, 125)).bg(bg),
                    ),
                ]))
            })
            .collect();
        frame.render_widget(
            List::new(items).block(Block::default().borders(Borders::LEFT | Borders::RIGHT)),
            body[0],
        );

        let (key, label, desc) = ENTRIES[cursor];
        let is_active_cur = key == cur_label;
        let action_hint = if is_active_cur {
            "Already active"
        } else {
            "Press Enter to apply"
        };
        let detail = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {label}"),
                Style::default()
                    .fg(Color::Rgb(210, 235, 255))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {desc}"),
                Style::default().fg(Color::Rgb(170, 205, 232)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {action_hint}"),
                Style::default().fg(if is_active_cur {
                    Color::Rgb(100, 200, 100)
                } else {
                    Color::Rgb(220, 180, 80)
                }),
            )),
        ])
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(65, 105, 145))),
        );
        frame.render_widget(detail, body[1]);

        frame.render_widget(Paragraph::new(picker_help_line(accent)), chunks[2]);
    }
}
