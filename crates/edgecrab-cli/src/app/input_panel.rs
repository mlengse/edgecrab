//! Input panel: textarea chrome, ghost hint, slash-command completion overlay.

use super::*;

impl App {
pub(super) fn render_input(&mut self, frame: &mut Frame, area: Rect) {
        // Configure the block before rendering so mode and pending-operator
        // feedback appear immediately on the current frame.
        let text = self.textarea_text();
        let block = if self.is_processing {
            // FP53: Animate the waiting title using the same spinner frame
            // as the status bar — zero extra state, perfect sync.
            let spinner =
                compact_spinner_frame(self.current_spinner_frame(), self.terminal_glyph_profile);
            let waiting_label = self
                .turn_activity
                .live_caption()
                .map(|caption| format!("{spinner} {caption}"))
                .unwrap_or_else(|| format!("{spinner} working…"));
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::Rgb(60, 60, 75))
                        .add_modifier(Modifier::DIM),
                )
                .title(self.input_panel_title(&waiting_label))
        } else if text.starts_with('/') {
            let cmd_name = text.split_whitespace().next().unwrap_or("");
            let is_valid = self.all_command_names.iter().any(|c| c == cmd_name);
            let border_color = if is_valid {
                Color::Cyan
            } else if cmd_name.len() > 1 {
                Color::Rgb(239, 83, 80)
            } else {
                self.theme.input_border.fg.unwrap_or(Color::White)
            };
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(self.input_panel_title(&self.theme.prompt_symbol))
        } else if text.starts_with('@') {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(self.input_panel_title(&self.theme.prompt_symbol))
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.input_border)
                .title(self.input_panel_title(&self.theme.prompt_symbol))
        };
        self.textarea.set_block(block);
        frame.render_widget(&self.textarea, area);

        // Ghost text overlay (Fish-style hint)
        if self.show_ghost_hint
            && matches!(self.editor_mode, InputEditorMode::Inline)
            && let Some(hint) = self.ghost_hint()
        {
            let (row, col) = self.textarea.cursor();
            let ghost_x = area.x + 1 + col as u16; // +1 for border
            let ghost_y = area.y + 1 + row as u16;
            if ghost_x < area.x + area.width - 1 {
                let max_width = (area.x + area.width - 1 - ghost_x) as usize;
                let display = edgecrab_core::safe_truncate(&hint, max_width);
                let ghost_area = Rect::new(ghost_x, ghost_y, display.len() as u16, 1);
                let ghost = Paragraph::new(Span::styled(
                    display.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
                frame.render_widget(ghost, ghost_area);
            }
        }

        // Completion overlay
        if matches!(self.editor_mode, InputEditorMode::Inline)
            && self.completion.active
            && !self.completion.candidates.is_empty()
        {
            let total_candidates = self.completion.candidates.len();
            let max_items = 8.min(total_candidates);
            let (scroll_start, scroll_end) = self.completion.visible_window(max_items);
            // +2 for top/bottom border, +1 for count footer
            let overlay_height = max_items as u16 + 3;
            let overlay_width = self
                .completion
                .candidates
                .iter()
                .map(|(cmd, desc)| {
                    let desc_len = if desc.is_empty() { 0 } else { 3 + desc.len() }; // " — desc"
                    cmd.len() + desc_len
                })
                .max()
                .unwrap_or(10) as u16
                + 4; // padding
            let overlay_width = overlay_width.clamp(24, area.width.saturating_sub(2));

            // Position above input area (with 1-row gap from input border)
            let overlay_y = area.y.saturating_sub(overlay_height);
            let overlay_x = area.x + 1;
            let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

            // Clear area behind overlay
            frame.render_widget(Clear, overlay_area);

            // Count indicator for the overlay title
            let sel_idx = self.completion.selected;
            let count_title = format!(
                " Commands {}/{} ",
                (sel_idx + 1).min(total_candidates),
                total_candidates
            );

            let items: Vec<ListItem> = self
                .completion
                .candidates
                .iter()
                .skip(scroll_start)
                .take(scroll_end.saturating_sub(scroll_start))
                .enumerate()
                .map(|(i, (cmd, desc))| {
                    let candidate_idx = scroll_start + i;
                    let is_selected = candidate_idx == self.completion.selected;
                    let bg = if is_selected {
                        Color::Rgb(55, 55, 75)
                    } else {
                        Color::Reset
                    };
                    let cmd_style = if is_selected {
                        Style::default()
                            .bg(bg)
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Rgb(200, 200, 210))
                    };
                    let desc_style = if is_selected {
                        Style::default().bg(bg).fg(Color::Rgb(140, 145, 165))
                    } else {
                        Style::default().fg(Color::Rgb(95, 100, 120))
                    };
                    let mut spans = vec![
                        selector_marker(is_selected, Color::Cyan, Some(bg)),
                        Span::styled(format!(" {cmd}"), cmd_style),
                    ];
                    if !desc.is_empty() {
                        spans.push(Span::styled(format!(" — {desc}"), desc_style));
                    }
                    ListItem::new(Line::from(spans))
                })
                .collect();

            let footer_line = if total_candidates > max_items {
                let hidden =
                    total_candidates.saturating_sub(scroll_end.saturating_sub(scroll_start));
                format!(
                    " Tab/↑↓ navigate  {} jump  +{} more ",
                    self.paging_key_hint_label(),
                    hidden
                )
            } else {
                " Tab/↑↓ navigate  Enter select  Esc cancel ".to_string()
            };

            // Split area: list body + footer
            let inner_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // list items
                    Constraint::Length(1), // footer hint
                ])
                .vertical_margin(1)
                .horizontal_margin(0)
                .split(overlay_area);

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Rgb(70, 75, 100)))
                        .title(count_title)
                        .title_style(Style::default().fg(Color::Rgb(140, 145, 165))),
                )
                .style(Style::default().bg(Color::Rgb(25, 25, 35)));
            frame.render_widget(list, overlay_area);

            // Render footer hint inside the border
            let footer_area = inner_chunks[1];
            let footer = Paragraph::new(Span::styled(
                footer_line,
                Style::default().fg(Color::Rgb(80, 85, 110)),
            ))
            .style(Style::default().bg(Color::Rgb(25, 25, 35)));
            frame.render_widget(footer, footer_area);
        }
    }
}
