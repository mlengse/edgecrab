//! Gateway diagnostics overlay (`/gateway diagnose`).

use super::*;

    /// Apply semantic TUI colors to a single line from the gateway diagnose report.
    ///
    /// WHY free associated function (no `self`): `render_diagnose_panel` maps over
    /// report lines and only needs the raw text + color palette — no app state.
    /// Keeping it here keeps all diagnose-overlay logic co-located.
pub fn colorize_diagnose_line<'a>(
        raw: &'a str,
        ok_color: Color,
        err_color: Color,
        warn_color: Color,
        heading_color: Color,
        dim_color: Color,
        accent: Color,
    ) -> Line<'a> {
        // Box-drawing borders (╔ ╚ ║ ═ etc.)
        if raw.contains('╔') || raw.contains('╚') || raw.contains('║') || raw.contains('╠')
        {
            return Line::from(Span::styled(
                raw,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ));
        }
        // Section dividers e.g. "── Title ──"
        let trimmed = raw.trim_start();
        if trimmed.starts_with("──") || trimmed.starts_with("─────") {
            return Line::from(Span::styled(raw, Style::default().fg(heading_color)));
        }
        // Status marker lines
        if raw.contains(" \u{2713} ") || raw.contains(" \u{2714} ") {
            // ✓ ✔
            return Line::from(Span::styled(raw, Style::default().fg(ok_color)));
        }
        if raw.contains(" \u{2717} ") || raw.contains(" \u{2718} ") {
            // ✗ ✘
            return Line::from(Span::styled(raw, Style::default().fg(err_color)));
        }
        if raw.contains(" \u{25cb} ") || raw.contains(" \u{25e6} ") {
            // ○ ◦  (offline / not configured)
            return Line::from(Span::styled(raw, Style::default().fg(dim_color)));
        }
        // Issues section entries  e.g. "[ERROR] ..."
        if trimmed.starts_with('[') && (trimmed.contains("] ") || trimmed.ends_with(']')) {
            let color = if trimmed.contains("[WARN") {
                warn_color
            } else {
                err_color
            };
            return Line::from(Span::styled(raw, Style::default().fg(color)));
        }
        // Fix suggestions
        if trimmed.starts_with("Fix:") || trimmed.starts_with("fix:") {
            return Line::from(Span::styled(
                raw,
                Style::default().fg(ok_color).add_modifier(Modifier::BOLD),
            ));
        }
        // Log severity keywords
        if raw.contains("ERROR") {
            return Line::from(Span::styled(raw, Style::default().fg(err_color)));
        }
        if raw.contains("WARN") {
            return Line::from(Span::styled(raw, Style::default().fg(warn_color)));
        }
        // Quick-action command lines e.g. "  edgecrab gateway start"
        if trimmed.starts_with("edgecrab ") {
            return Line::from(Span::styled(raw, Style::default().fg(accent)));
        }
        // Default: plain text, let the paragraph style apply
        Line::from(raw)
    }

impl App {
    /// Render the full-screen Gateway Diagnostics overlay.
    ///
    /// DRY: re-uses `browser_overlay_chunks` and the established accent palette
    /// so it inherits the same visual language as other overlays with zero
    /// extra styling primitives.
pub(super) fn render_diagnose_panel(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        // Accent: teal — distinct from the warm-amber log/model overlays.
        let accent = Color::Rgb(80, 220, 200);
        let heading_color = Color::Rgb(255, 220, 100);
        let ok_color = Color::Rgb(100, 220, 100);
        let err_color = Color::Rgb(255, 100, 80);
        let warn_color = Color::Rgb(255, 180, 50);
        let dim_color = Color::Rgb(130, 130, 140);
        let bg = Color::Rgb(14, 20, 26);

        // ── Layout ───────────────────────────────────────────────────────────
        // 3 rows: header bar (3) / body / footer (1)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        // ── Header ───────────────────────────────────────────────────────────
        let title_text = if self.diagnose_panel.refresh_in_flight {
            "  Gateway Diagnostics  ·  refreshing…"
        } else {
            "  Gateway Diagnostics  ·  /gateway diagnose"
        };
        let header = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(accent))
            .title(Span::styled(
                title_text,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(bg));
        frame.render_widget(header, chunks[0]);

        // ── Body: colorized report lines ─────────────────────────────────────
        let report_lines: Vec<Line> = self
            .diagnose_panel
            .report
            .lines()
            .map(|raw| {
                colorize_diagnose_line(
                    raw,
                    ok_color,
                    err_color,
                    warn_color,
                    heading_color,
                    dim_color,
                    accent,
                )
            })
            .collect();

        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        let scroll = self
            .diagnose_panel
            .scroll
            .min(self.diagnose_panel.total_lines.saturating_sub(1));

        let paragraph = Paragraph::new(report_lines)
            .scroll((scroll as u16, 0))
            .style(Style::default().bg(bg).fg(Color::Rgb(225, 220, 210)))
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::Rgb(50, 70, 80)))
                    .style(Style::default().bg(bg)),
            )
            .wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(paragraph, chunks[1]);

        // Scroll position indicator (top-right of body border)
        let total = self.diagnose_panel.total_lines;
        let bottom = (scroll + visible_height).min(total);
        let pos_text = format!(" {scroll}–{bottom}/{total} ");
        let pos_x = chunks[1]
            .x
            .saturating_add(chunks[1].width.saturating_sub(pos_text.len() as u16 + 1));
        let pos_area = Rect {
            x: pos_x,
            y: chunks[1].y + chunks[1].height.saturating_sub(1),
            width: pos_text.len() as u16,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled(pos_text, Style::default().fg(dim_color))),
            pos_area,
        );

        // ── Footer: key hints ────────────────────────────────────────────────
        let help = Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓ ", Style::default().fg(accent)),
            Span::styled("scroll  ", Style::default().fg(dim_color)),
            Span::styled("PgUp/PgDn ", Style::default().fg(accent)),
            Span::styled("page  ", Style::default().fg(dim_color)),
            Span::styled("R ", Style::default().fg(accent)),
            Span::styled("refresh  ", Style::default().fg(dim_color)),
            Span::styled("Esc/Q ", Style::default().fg(accent)),
            Span::styled("close", Style::default().fg(dim_color)),
        ]));
        frame.render_widget(help, chunks[2]);
    }
    /// Handle keyboard input while the Gateway Diagnostics overlay is open.
pub(super) fn handle_diagnose_panel_key(&mut self, key: event::KeyEvent) {
        let page = self.output_area_height.max(4).saturating_sub(2) as usize;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.diagnose_panel.active = false;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                self.diagnose_panel.scroll = self.diagnose_panel.scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                let max = self.diagnose_panel.total_lines.saturating_sub(1);
                self.diagnose_panel.scroll = (self.diagnose_panel.scroll + 1).min(max);
            }
            KeyCode::PageUp => {
                self.diagnose_panel.scroll = self.diagnose_panel.scroll.saturating_sub(page);
            }
            KeyCode::PageDown => {
                let max = self.diagnose_panel.total_lines.saturating_sub(1);
                self.diagnose_panel.scroll = (self.diagnose_panel.scroll + page).min(max);
            }
            KeyCode::Home => {
                self.diagnose_panel.scroll = 0;
            }
            KeyCode::End => {
                self.diagnose_panel.scroll = self.diagnose_panel.total_lines.saturating_sub(1);
            }
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::F(5) => {
                // Refresh: re-run diagnostics and stay in overlay.
                self.handle_gateway_control("diagnose".into());
            }
            _ => {}
        }
        self.needs_redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn colorize_marks_bracket_errors_red() {
        let line = colorize_diagnose_line(
            "  [ERROR] gateway offline",
            Color::Green,
            Color::Red,
            Color::Yellow,
            Color::White,
            Color::Gray,
            Color::Cyan,
        );
        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
    }
}
