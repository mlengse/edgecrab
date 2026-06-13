//! Log and session browser overlays (F5 / `/sessions`, log inspector).

use super::*;

impl App {

    pub(super) fn render_log_browser(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let accent = Color::Rgb(255, 196, 120);
        let chunks = Self::browser_overlay_chunks(area);
        let body = Self::browser_body_chunks(chunks[1]);
        self.render_browser_header(
            frame,
            chunks[0],
            &self.log_browser.query,
            BrowserChrome {
                title: "Log Browser",
                placeholder: "Search log files by name, type, preview text, or size.",
                icon: "◷",
                icon_color: accent,
                border_color: accent,
            },
        );

        let filtered = &self.log_browser.filtered;
        let selected = self.log_browser.selected;
        let max_visible = Self::browser_list_visible_rows(body[0], true);
        let scroll_start = Self::browser_scroll_start(selected, max_visible);

        let items: Vec<ListItem> = if filtered.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  No log files matched the current filter.",
                Style::default().fg(Color::Rgb(120, 120, 135)),
            )))]
        } else {
            filtered
                .iter()
                .skip(scroll_start)
                .take(max_visible)
                .enumerate()
                .map(|(vis_idx, &entry_idx)| {
                    let entry = &self.log_browser.items[entry_idx];
                    let is_selected = vis_idx + scroll_start == selected;
                    let bg = if is_selected {
                        Color::Rgb(46, 34, 20)
                    } else {
                        Color::Rgb(18, 22, 28)
                    };
                    let tag_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(230, 188, 140))
                    } else {
                        Style::default().fg(Color::Rgb(150, 126, 100))
                    };
                    let title_style = if is_selected {
                        Style::default()
                            .bg(bg)
                            .fg(accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(232, 226, 214))
                    };
                    let detail_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(208, 182, 150))
                    } else {
                        Style::default().fg(Color::Rgb(140, 136, 128))
                    };
                    let preview_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(220, 204, 184))
                    } else {
                        Style::default().fg(Color::Rgb(150, 150, 150))
                    };

                    ListItem::new(Line::from(vec![
                        selector_marker(is_selected, accent, Some(bg)),
                        Span::styled(format!("  {:<8}", entry.kind), tag_style),
                        Span::styled(unicode_trunc(&entry.name, 24), title_style),
                        Span::raw("  "),
                        Span::styled(unicode_trunc(&entry.detail, 30), detail_style),
                        Span::raw("  "),
                        Span::styled(unicode_trunc(&entry.preview, 28), preview_style),
                    ]))
                })
                .collect()
        };
        let list_border_style = if self.log_browser_pane.focus == SplitPaneFocus::List {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(60, 80, 84))
        };
        frame.render_widget(
            List::new(items)
                .style(Style::default().bg(Color::Rgb(18, 22, 28)))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(list_border_style)
                        .title(" Files "),
                ),
            body[0],
        );

        let detail_lines = if let Some(entry) = self.log_browser.current() {
            let mut lines = vec![Line::from(vec![
                Span::styled(
                    format!("{} ", entry.kind),
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
                Span::raw(entry.name.clone()),
            ])];
            lines.push(Line::from(""));
            lines.extend(
                entry
                    .detail_view
                    .lines()
                    .map(|line| Line::from(line.to_string())),
            );
            lines
        } else {
            default_log_browser_detail_lines()
        };

        self.render_scrollable_browser_detail(
            frame,
            body[1],
            ScrollableDetailChrome {
                title: "Details",
                border_color: accent,
                focused: self.log_browser_pane.focus == SplitPaneFocus::Detail,
                requested_scroll: self.log_browser_pane.scroll,
            },
            detail_lines.clone(),
        );
        if self.detail_fullscreen_active(DetailSurface::LogBrowser) {
            self.render_fullscreen_browser_detail(
                frame,
                area,
                FullscreenBrowserChrome {
                    query: &self.log_browser.query,
                    header: BrowserChrome {
                        title: "Log Browser",
                        placeholder: "Search log files by name, type, preview text, or size.",
                        icon: "◷",
                        icon_color: accent,
                        border_color: accent,
                    },
                    detail: ScrollableDetailChrome {
                        title: "Details",
                        border_color: accent,
                        focused: true,
                        requested_scroll: self.detail_fullscreen_scroll(DetailSurface::LogBrowser),
                    },
                    help: Line::from(vec![
                        Span::styled(" ↑↓ ", Style::default().fg(accent)),
                        Span::styled("change file  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("type ", Style::default().fg(accent)),
                        Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
                        self.paging_key_help_span(accent),
                        Span::styled("scroll detail  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Enter ", Style::default().fg(accent)),
                        Span::styled("inspect  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("F ", Style::default().fg(accent)),
                        Span::styled("follow  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("1-5 ", Style::default().fg(accent)),
                        Span::styled("level  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Z ", Style::default().fg(accent)),
                        Span::styled("split view  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Esc ", Style::default().fg(accent)),
                        Span::styled("close", Style::default().fg(Color::DarkGray)),
                    ]),
                },
                detail_lines,
            );
            return;
        }

        let note = self.log_browser_status_note.as_deref().unwrap_or(
            "Lowercase refines the filter. Enter opens an entry inspector for the selected file tail.",
        );
        let help = Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓ ", Style::default().fg(accent)),
            Span::styled("files  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab ", Style::default().fg(accent)),
            Span::styled("focus pane  ", Style::default().fg(Color::DarkGray)),
            Span::styled("type ", Style::default().fg(accent)),
            Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
            self.paging_key_help_span(accent),
            Span::styled("page or scroll  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter ", Style::default().fg(accent)),
            Span::styled("inspect  ", Style::default().fg(Color::DarkGray)),
            Span::styled("R ", Style::default().fg(accent)),
            Span::styled("reload  ", Style::default().fg(Color::DarkGray)),
            Span::styled("F ", Style::default().fg(accent)),
            Span::styled("follow  ", Style::default().fg(Color::DarkGray)),
            Span::styled("1-5 ", Style::default().fg(accent)),
            Span::styled("level  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Z ", Style::default().fg(accent)),
            Span::styled("detail  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(accent)),
            Span::styled("close  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} file(s) · {} pane · {}",
                    filtered.len(),
                    match self.log_browser_pane.focus {
                        SplitPaneFocus::List => "list",
                        SplitPaneFocus::Detail => "detail",
                    },
                    note
                ),
                Style::default().fg(Color::Rgb(130, 120, 105)),
            ),
        ]));
        frame.render_widget(help, chunks[2]);
    }

    pub(super) fn build_log_inspector_detail_lines(&self) -> Vec<Line<'static>> {
        let mut detail_lines = Vec::new();
        if let Some(file) = self.log_inspector.file.as_ref() {
            detail_lines.push(Line::from(vec![
                Span::styled(
                    file.name.clone(),
                    Style::default()
                        .fg(Color::Rgb(255, 196, 120))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  ({})", file.kind)),
            ]));
            detail_lines.push(Line::from(format!(
                "{} · modified {}",
                file.size_label, file.modified_label
            )));
            detail_lines.push(Line::from(format!("Path: {}", file.path.display())));
            detail_lines.push(Line::from(format!(
                "Live follow: {}",
                self.log_follow.badge()
            )));
            if !file.preview.is_empty() {
                detail_lines.push(Line::from(format!("Tail preview: {}", file.preview)));
            }
            if !self.log_inspector.selector.query.trim().is_empty() {
                detail_lines.push(Line::from(format!(
                    "Local filter: {}",
                    self.log_inspector.selector.query
                )));
            }
            detail_lines.push(Line::from(""));

            if let Some(entry) = self.log_inspector.selector.current() {
                detail_lines.push(Line::from(Span::styled(
                    "Selected entry",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                detail_lines.push(Line::from(format!(
                    "{} · {}",
                    entry.level_label, entry.timestamp
                )));
                detail_lines.push(Line::from(format!("Summary: {}", entry.summary)));
                detail_lines.push(Line::from(""));
                for line in entry.detail.lines() {
                    detail_lines.push(Line::from(line.to_string()));
                }
            } else {
                detail_lines.push(Line::from(
                    "No log entry is selected. Clear the filter or move the cursor to inspect the tail.",
                ));
            }
        }
        detail_lines
    }

    pub(super) fn render_log_inspector(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let accent = Color::Rgb(255, 196, 120);
        let chunks = Self::browser_overlay_chunks(area);
        let body = Self::browser_body_chunks(chunks[1]);
        self.render_browser_header(
            frame,
            chunks[0],
            &self.log_inspector.selector.query,
            BrowserChrome {
                title: "Log Inspector",
                placeholder: "Filter the selected file tail by level, timestamp, message, or stacktrace text.",
                icon: "⌘",
                icon_color: accent,
                border_color: accent,
            },
        );

        let filtered = &self.log_inspector.selector.filtered;
        let selected = self.log_inspector.selector.selected;
        let max_visible = Self::browser_list_visible_rows(body[0], true);
        let scroll_start = Self::browser_scroll_start(selected, max_visible);

        let items: Vec<ListItem> = if filtered.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  No log entries matched the current filter.",
                Style::default().fg(Color::Rgb(120, 120, 135)),
            )))]
        } else {
            filtered
                .iter()
                .skip(scroll_start)
                .take(max_visible)
                .enumerate()
                .map(|(vis_idx, &entry_idx)| {
                    let entry = &self.log_inspector.selector.items[entry_idx];
                    let is_selected = vis_idx + scroll_start == selected;
                    let bg = if is_selected {
                        Color::Rgb(46, 34, 20)
                    } else {
                        Color::Rgb(18, 22, 28)
                    };
                    let tag_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(255, 210, 150))
                    } else {
                        Style::default().fg(Color::Rgb(170, 140, 112))
                    };
                    let title_style = if is_selected {
                        Style::default()
                            .bg(bg)
                            .fg(accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(225, 220, 210))
                    };
                    let detail_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(215, 194, 168))
                    } else {
                        Style::default().fg(Color::Rgb(145, 145, 145))
                    };

                    ListItem::new(Line::from(vec![
                        selector_marker(is_selected, accent, Some(bg)),
                        Span::styled(format!("  {:<6}", entry.level_label), tag_style),
                        Span::styled(unicode_trunc(&entry.timestamp, 19), detail_style),
                        Span::raw("  "),
                        Span::styled(unicode_trunc(&entry.summary, 72), title_style),
                    ]))
                })
                .collect()
        };
        let list_border_style = if self.log_inspector.pane.focus == SplitPaneFocus::List {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(60, 80, 84))
        };
        frame.render_widget(
            List::new(items)
                .style(Style::default().bg(Color::Rgb(18, 22, 28)))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(list_border_style)
                        .title(" Entries "),
                ),
            body[0],
        );

        let detail_lines = self.build_log_inspector_detail_lines();
        self.render_scrollable_browser_detail(
            frame,
            body[1],
            ScrollableDetailChrome {
                title: "Details",
                border_color: accent,
                focused: self.log_inspector.pane.focus == SplitPaneFocus::Detail,
                requested_scroll: self.log_inspector.pane.scroll,
            },
            detail_lines.clone(),
        );
        if self.detail_fullscreen_active(DetailSurface::LogInspector) {
            self.render_fullscreen_browser_detail(
                frame,
                area,
                FullscreenBrowserChrome {
                    query: &self.log_inspector.selector.query,
                    header: BrowserChrome {
                        title: "Log Inspector",
                        placeholder: "Filter the selected file tail by level, timestamp, message, or stacktrace text.",
                        icon: "⌘",
                        icon_color: accent,
                        border_color: accent,
                    },
                    detail: ScrollableDetailChrome {
                        title: "Details",
                        border_color: accent,
                        focused: true,
                        requested_scroll: self.detail_fullscreen_scroll(DetailSurface::LogInspector),
                    },
                    help: Line::from(vec![
                        Span::styled(" ↑↓ ", Style::default().fg(accent)),
                        Span::styled("change entry  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("type ", Style::default().fg(accent)),
                        Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
                        self.paging_key_help_span(accent),
                        Span::styled("scroll detail  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("B ", Style::default().fg(accent)),
                        Span::styled("back  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("R ", Style::default().fg(accent)),
                        Span::styled("reload  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("F ", Style::default().fg(accent)),
                        Span::styled("follow  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("1-5 ", Style::default().fg(accent)),
                        Span::styled("level  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Z ", Style::default().fg(accent)),
                        Span::styled("split view  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Esc ", Style::default().fg(accent)),
                        Span::styled("close", Style::default().fg(Color::DarkGray)),
                    ]),
                },
                detail_lines,
            );
            return;
        }

        let help = Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓ ", Style::default().fg(accent)),
            Span::styled("entries  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab ", Style::default().fg(accent)),
            Span::styled("focus pane  ", Style::default().fg(Color::DarkGray)),
            Span::styled("type ", Style::default().fg(accent)),
            Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
            self.paging_key_help_span(accent),
            Span::styled("page or scroll  ", Style::default().fg(Color::DarkGray)),
            Span::styled("B ", Style::default().fg(accent)),
            Span::styled("back  ", Style::default().fg(Color::DarkGray)),
            Span::styled("R ", Style::default().fg(accent)),
            Span::styled("reload  ", Style::default().fg(Color::DarkGray)),
            Span::styled("F ", Style::default().fg(accent)),
            Span::styled("follow  ", Style::default().fg(Color::DarkGray)),
            Span::styled("1-5 ", Style::default().fg(accent)),
            Span::styled("level  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Z ", Style::default().fg(accent)),
            Span::styled("detail  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(accent)),
            Span::styled("close  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} entry(s) · {} pane · {}",
                    filtered.len(),
                    match self.log_inspector.pane.focus {
                        SplitPaneFocus::List => "list",
                        SplitPaneFocus::Detail => "detail",
                    },
                    self.log_follow.badge()
                ),
                Style::default().fg(Color::Rgb(130, 120, 105)),
            ),
        ]));
        frame.render_widget(help, chunks[2]);
    }

    pub(super) fn build_session_inspector_detail_lines(&self) -> Vec<Line<'static>> {
        let mut detail_lines = Vec::new();
        let selector = &self.session_inspector.selector;

        if let Some(session) = self.session_inspector.session.as_ref() {
            detail_lines.push(Line::from(vec![
                Span::styled(
                    session.title.clone(),
                    Style::default()
                        .fg(Color::Rgb(120, 215, 185))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  ({})", session.source)),
            ]));
            detail_lines.push(Line::from(format!(
                "{} · {} messages · started {} · last active {}",
                session.model,
                session.message_count,
                session.started_label,
                session.last_active_label
            )));
            detail_lines.push(Line::from(format!("Session ID: {}", session.id)));
            if !session.preview.is_empty() {
                detail_lines.push(Line::from(format!("Preview: {}", session.preview)));
            }
            if let (Some(role), Some(snippet)) = (
                session.matched_role.as_deref(),
                session.matched_snippet.as_deref(),
            ) {
                detail_lines.push(Line::from(format!(
                    "Opened from browser match: {role} -> {snippet}"
                )));
            }
            if !selector.query.trim().is_empty() {
                detail_lines.push(Line::from(format!("Local filter: {}", selector.query)));
            }
            if session.is_live {
                detail_lines.push(Line::from("Mode: live in-memory session debugger"));
            }
            for line in &session.debug_lines {
                detail_lines.push(Line::from(line.clone()));
            }
            detail_lines.push(Line::from(""));

            if let Some(entry) = selector.current() {
                detail_lines.push(Line::from(Span::styled(
                    "Selected message",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                detail_lines.push(Line::from(vec![
                    Span::styled(
                        entry.headline.clone(),
                        Style::default()
                            .fg(Color::Rgb(120, 215, 185))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("  {}", entry.meta)),
                ]));
                detail_lines.push(Line::from(""));

                let content = entry.message.text_content();
                if !content.trim().is_empty() {
                    detail_lines.push(Line::from(Span::styled(
                        "Content",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    for line in content.lines() {
                        detail_lines.push(Line::from(line.to_string()));
                    }
                } else {
                    detail_lines.push(Line::from("(No text content stored for this message.)"));
                }

                if let Some(reasoning) = entry
                    .message
                    .reasoning
                    .as_deref()
                    .filter(|reasoning| !reasoning.trim().is_empty())
                {
                    detail_lines.push(Line::from(""));
                    detail_lines.push(Line::from(Span::styled(
                        "Reasoning",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    for line in reasoning.lines() {
                        detail_lines.push(Line::from(line.to_string()));
                    }
                }

                if let Some(tool_calls) = entry
                    .message
                    .tool_calls
                    .as_ref()
                    .filter(|tool_calls| !tool_calls.is_empty())
                {
                    detail_lines.push(Line::from(""));
                    detail_lines.push(Line::from(Span::styled(
                        "Tool Calls",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    for call in tool_calls {
                        detail_lines.push(Line::from(format!(
                            "- {}  ({})",
                            call.function.name, call.id
                        )));
                    }
                }
            } else {
                detail_lines.push(Line::from(
                    "No message is selected. Clear the filter or move the cursor to inspect the timeline.",
                ));
            }
        }

        detail_lines
    }

    /// Render the session browser overlay (activated by F5 or `/sessions`).
    pub(super) fn render_session_browser(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let chunks = Self::browser_overlay_chunks(area);
        let body = Self::browser_body_chunks(chunks[1]);
        self.render_browser_header(
            frame,
            chunks[0],
            &self.session_browser.query,
            BrowserChrome {
                title: "Session Browser",
                placeholder: "Search by title, id, source, model, or any indexed message text.",
                icon: "⏱",
                icon_color: Color::Rgb(110, 190, 255),
                border_color: Color::Rgb(110, 190, 255),
            },
        );

        let filtered = &self.session_browser.filtered;
        let selected = self.session_browser.selected;
        let max_visible = Self::browser_list_visible_rows(body[0], true);
        let scroll_start = Self::browser_scroll_start(selected, max_visible);

        let items: Vec<ListItem> = if filtered.is_empty() {
            let empty_text = if self.session_browser.query.trim().is_empty() {
                "  No saved sessions are available."
            } else {
                "  No sessions matched this query."
            };
            vec![ListItem::new(Line::from(Span::styled(
                empty_text.to_string(),
                Style::default().fg(Color::Rgb(120, 120, 135)),
            )))]
        } else {
            filtered
                .iter()
                .skip(scroll_start)
                .take(max_visible)
                .enumerate()
                .map(|(vis_idx, &entry_idx)| {
                    let entry = &self.session_browser.items[entry_idx];
                    let is_selected = vis_idx + scroll_start == selected;
                    let bg = if is_selected {
                        Color::Rgb(20, 34, 48)
                    } else {
                        Color::Rgb(18, 22, 28)
                    };
                    let source_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(140, 170, 210))
                    } else {
                        Style::default().fg(Color::Rgb(95, 115, 145))
                    };
                    let title_style = if is_selected {
                        Style::default()
                            .bg(bg)
                            .fg(Color::Rgb(125, 215, 255))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(210, 225, 235))
                    };
                    let detail_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(145, 175, 188))
                    } else {
                        Style::default().fg(Color::Rgb(118, 135, 150))
                    };
                    let preview_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(168, 188, 196))
                    } else {
                        Style::default().fg(Color::Rgb(132, 146, 156))
                    };

                    ListItem::new(Line::from(vec![
                        selector_marker(is_selected, Color::Rgb(110, 190, 255), Some(bg)),
                        Span::styled(format!("  {:<10}", entry.source), source_style),
                        Span::styled(unicode_trunc(&entry.display, 28), title_style),
                        Span::raw("  "),
                        Span::styled(unicode_trunc(&entry.detail, 28), detail_style),
                        Span::raw("  "),
                        Span::styled(unicode_trunc(&entry.preview, 34), preview_style),
                    ]))
                })
                .collect()
        };
        let list_border_style = if self.session_browser_pane.focus == SplitPaneFocus::List {
            Style::default()
                .fg(Color::Rgb(110, 190, 255))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(60, 80, 84))
        };
        frame.render_widget(
            List::new(items)
                .style(Style::default().bg(Color::Rgb(18, 22, 28)))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(list_border_style)
                        .title(" Sessions "),
                ),
            body[0],
        );

        let mut detail_lines = Vec::new();
        if let Some(entry) = self.session_browser.current() {
            detail_lines.push(Line::from(vec![
                Span::styled(
                    format!("{} ", entry.source),
                    Style::default()
                        .fg(Color::Rgb(110, 190, 255))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(entry.display.clone()),
            ]));
            detail_lines.push(Line::from(""));
            for line in entry.detail_view.lines() {
                detail_lines.push(Line::from(line.to_string()));
            }
        } else if self.session_browser.query.trim().is_empty() {
            detail_lines.push(Line::from(Span::styled(
                "Session Browser",
                Style::default()
                    .fg(Color::Rgb(110, 190, 255))
                    .add_modifier(Modifier::BOLD),
            )));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(
                "Browse recent sessions on the left and inspect metadata on the right.",
            ));
            detail_lines.push(Line::from(
                "Search matches local metadata instantly and also checks the full indexed message archive.",
            ));
        } else {
            detail_lines.push(Line::from("No results for the current query."));
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(
                "Try a title fragment, model name, source like cli or telegram, or terms from the conversation itself.",
            ));
        }
        self.render_scrollable_browser_detail(
            frame,
            body[1],
            ScrollableDetailChrome {
                title: "Details",
                border_color: Color::Rgb(110, 190, 255),
                focused: self.session_browser_pane.focus == SplitPaneFocus::Detail,
                requested_scroll: self.session_browser_pane.scroll,
            },
            detail_lines.clone(),
        );
        if self.detail_fullscreen_active(DetailSurface::SessionBrowser) {
            self.render_fullscreen_browser_detail(
                frame,
                area,
                FullscreenBrowserChrome {
                    query: &self.session_browser.query,
                    header: BrowserChrome {
                        title: "Session Browser",
                        placeholder:
                            "Search by title, id, source, model, or any indexed message text.",
                        icon: "⏱",
                        icon_color: Color::Rgb(110, 190, 255),
                        border_color: Color::Rgb(110, 190, 255),
                    },
                    detail: ScrollableDetailChrome {
                        title: "Details",
                        border_color: Color::Rgb(110, 190, 255),
                        focused: true,
                        requested_scroll: self
                            .detail_fullscreen_scroll(DetailSurface::SessionBrowser),
                    },
                    help: Line::from(vec![
                        Span::styled(" ↑↓ ", Style::default().fg(Color::Rgb(110, 190, 255))),
                        Span::styled("change item  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("type ", Style::default().fg(Color::Rgb(110, 190, 255))),
                        Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
                        self.paging_key_help_span(Color::Rgb(110, 190, 255)),
                        Span::styled("scroll detail  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Enter ", Style::default().fg(Color::Rgb(110, 190, 255))),
                        Span::styled("inspect  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("R ", Style::default().fg(Color::Rgb(110, 190, 255))),
                        Span::styled("resume  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Z ", Style::default().fg(Color::Rgb(110, 190, 255))),
                        Span::styled("split view  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Esc ", Style::default().fg(Color::Rgb(110, 190, 255))),
                        Span::styled("close  ", Style::default().fg(Color::DarkGray)),
                    ]),
                },
                detail_lines,
            );
            return;
        }

        let note = self.session_browser_status_note.as_deref().unwrap_or(
            "Lowercase keeps typing in the filter. Uppercase shortcuts trigger actions.",
        );
        let help = Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓ ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("list  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("focus pane  ", Style::default().fg(Color::DarkGray)),
            Span::styled("type ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
            self.paging_key_help_span(Color::Rgb(110, 190, 255)),
            Span::styled("page or scroll  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("inspect  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Z ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("detail  ", Style::default().fg(Color::DarkGray)),
            Span::styled("R ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("resume  ", Style::default().fg(Color::DarkGray)),
            Span::styled("D ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("delete  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(Color::Rgb(110, 190, 255))),
            Span::styled("close  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} visible · {} pane · {}",
                    filtered.len(),
                    match self.session_browser_pane.focus {
                        SplitPaneFocus::List => "list",
                        SplitPaneFocus::Detail => "detail",
                    },
                    note
                ),
                Style::default().fg(Color::Rgb(100, 120, 130)),
            ),
        ]));
        frame.render_widget(help, chunks[2]);
    }

    pub(super) fn render_session_inspector(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let chunks = Self::browser_overlay_chunks(area);
        let body = Self::browser_body_chunks(chunks[1]);
        self.render_browser_header(
            frame,
            chunks[0],
            &self.session_inspector.selector.query,
            BrowserChrome {
                title: "Session Inspector",
                placeholder:
                    "Filter this timeline by role, content, tool ids, tool names, or reasoning text.",
                icon: "⌕",
                icon_color: Color::Rgb(120, 215, 185),
                border_color: Color::Rgb(120, 215, 185),
            },
        );

        let selector = &self.session_inspector.selector;
        let filtered = &selector.filtered;
        let selected = selector.selected;
        let max_visible = Self::browser_list_visible_rows(body[0], true);
        let scroll_start = Self::browser_scroll_start(selected, max_visible);

        let items: Vec<ListItem> = if filtered.is_empty() {
            let empty_text = if selector.query.trim().is_empty() {
                "  No messages were stored for this session."
            } else {
                "  No messages in this session matched the current filter."
            };
            vec![ListItem::new(Line::from(Span::styled(
                empty_text.to_string(),
                Style::default().fg(Color::Rgb(120, 120, 135)),
            )))]
        } else {
            filtered
                .iter()
                .skip(scroll_start)
                .take(max_visible)
                .enumerate()
                .map(|(vis_idx, &entry_idx)| {
                    let entry = &selector.items[entry_idx];
                    let is_selected = vis_idx + scroll_start == selected;
                    let bg = if is_selected {
                        Color::Rgb(24, 36, 34)
                    } else {
                        Color::Rgb(18, 24, 24)
                    };
                    let role_color = match entry.role_label.as_str() {
                        "user" => Color::Rgb(110, 190, 255),
                        "assistant" => Color::Rgb(120, 215, 185),
                        "tool" => Color::Rgb(235, 190, 105),
                        "system" => Color::Rgb(190, 145, 240),
                        _ => Color::Rgb(190, 190, 190),
                    };
                    let index_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(145, 170, 170))
                    } else {
                        Style::default().fg(Color::Rgb(92, 112, 112))
                    };
                    let role_style = if is_selected {
                        Style::default()
                            .bg(bg)
                            .fg(role_color)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(role_color)
                    };
                    let preview_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(218, 230, 228))
                    } else {
                        Style::default().fg(Color::Rgb(188, 198, 196))
                    };
                    let meta_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(145, 175, 170))
                    } else {
                        Style::default().fg(Color::Rgb(110, 136, 132))
                    };

                    ListItem::new(Line::from(vec![
                        selector_marker(is_selected, role_color, Some(bg)),
                        Span::styled(format!("  {:>3}", entry.index + 1), index_style),
                        Span::raw("  "),
                        Span::styled(format!("{:<10}", entry.role_label), role_style),
                        Span::styled(unicode_trunc(&entry.preview, 38), preview_style),
                        Span::raw("  "),
                        Span::styled(unicode_trunc(&entry.meta, 22), meta_style),
                    ]))
                })
                .collect()
        };
        let list_border_style = if self.session_inspector.pane.focus == SplitPaneFocus::List {
            Style::default()
                .fg(Color::Rgb(120, 215, 185))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(60, 80, 84))
        };
        frame.render_widget(
            List::new(items)
                .style(Style::default().bg(Color::Rgb(18, 24, 24)))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(list_border_style)
                        .title(" Timeline "),
                ),
            body[0],
        );

        let detail_lines = self.build_session_inspector_detail_lines();
        self.render_scrollable_browser_detail(
            frame,
            body[1],
            ScrollableDetailChrome {
                title: "Details",
                border_color: Color::Rgb(120, 215, 185),
                focused: self.session_inspector.pane.focus == SplitPaneFocus::Detail,
                requested_scroll: self.session_inspector.pane.scroll,
            },
            detail_lines.clone(),
        );
        if self.detail_fullscreen_active(DetailSurface::SessionInspector) {
            self.render_fullscreen_browser_detail(
                frame,
                area,
                FullscreenBrowserChrome {
                    query: &self.session_inspector.selector.query,
                    header: BrowserChrome {
                        title: "Session Inspector",
                        placeholder:
                            "Filter this timeline by role, content, tool ids, tool names, or reasoning text.",
                        icon: "⌕",
                        icon_color: Color::Rgb(120, 215, 185),
                        border_color: Color::Rgb(120, 215, 185),
                    },
                    detail: ScrollableDetailChrome {
                        title: "Details",
                        border_color: Color::Rgb(120, 215, 185),
                        focused: true,
                        requested_scroll: self
                            .detail_fullscreen_scroll(DetailSurface::SessionInspector),
                    },
                    help: Line::from(vec![
                        Span::styled(" ↑↓ ", Style::default().fg(Color::Rgb(120, 215, 185))),
                        Span::styled("change item  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("type ", Style::default().fg(Color::Rgb(120, 215, 185))),
                        Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
                        self.paging_key_help_span(Color::Rgb(120, 215, 185)),
                        Span::styled("scroll detail  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("R ", Style::default().fg(Color::Rgb(120, 215, 185))),
                        Span::styled("resume saved  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Z ", Style::default().fg(Color::Rgb(120, 215, 185))),
                        Span::styled("split view  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Esc ", Style::default().fg(Color::Rgb(120, 215, 185))),
                        Span::styled("back  ", Style::default().fg(Color::DarkGray)),
                    ]),
                },
                detail_lines,
            );
            return;
        }

        let help = Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓ ", Style::default().fg(Color::Rgb(120, 215, 185))),
            Span::styled("timeline  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab ", Style::default().fg(Color::Rgb(120, 215, 185))),
            Span::styled("focus pane  ", Style::default().fg(Color::DarkGray)),
            Span::styled("type ", Style::default().fg(Color::Rgb(120, 215, 185))),
            Span::styled("filter  ", Style::default().fg(Color::DarkGray)),
            self.paging_key_help_span(Color::Rgb(120, 215, 185)),
            Span::styled("page or scroll  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Z ", Style::default().fg(Color::Rgb(120, 215, 185))),
            Span::styled("detail  ", Style::default().fg(Color::DarkGray)),
            Span::styled("B ", Style::default().fg(Color::Rgb(120, 215, 185))),
            Span::styled("back  ", Style::default().fg(Color::DarkGray)),
            Span::styled("R ", Style::default().fg(Color::Rgb(120, 215, 185))),
            Span::styled("resume saved  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(Color::Rgb(120, 215, 185))),
            Span::styled("back  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} visible · {} pane",
                    filtered.len(),
                    match self.session_inspector.pane.focus {
                        SplitPaneFocus::List => "list",
                        SplitPaneFocus::Detail => "detail",
                    }
                ),
                Style::default().fg(Color::Rgb(100, 120, 120)),
            ),
        ]));
        frame.render_widget(help, chunks[2]);
    }
}
