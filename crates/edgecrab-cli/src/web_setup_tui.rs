//! Single-screen `/web` configurator — one list, three keys.
//!
//! Enter = primary · Space = fallback · a = auto · Esc = close

use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json::Value;

use crate::web_command::WEB_ACCENT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSetupScreen {
    Configure,
    ConfirmAuto,
}

#[derive(Debug, Clone)]
pub struct WebSetupTui {
    pub active: bool,
    pub screen: WebSetupScreen,
    pub list_cursor: usize,
    pub provider_rows: Vec<Value>,
    pub search_backend_ids: Vec<String>,
    pub chain_primary_cursor: usize,
    pub chain_fallback_order: Vec<String>,
    pub toast: Option<String>,
    config_path: PathBuf,
}

impl WebSetupTui {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            active: false,
            screen: WebSetupScreen::Configure,
            list_cursor: 0,
            provider_rows: Vec::new(),
            search_backend_ids: Vec::new(),
            chain_primary_cursor: 0,
            chain_fallback_order: Vec::new(),
            toast: None,
            config_path,
        }
    }

    pub fn open(&mut self) {
        self.reload();
        self.screen = WebSetupScreen::Configure;
        self.toast = None;
        self.active = true;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.screen = WebSetupScreen::Configure;
    }

    fn reload(&mut self) {
        self.provider_rows = edgecrab_tools::web_provider_picker_rows();
        self.search_backend_ids = self
            .provider_rows
            .iter()
            .filter(|r| r["supports_search"].as_bool() == Some(true))
            .filter(|r| {
                let id = r["id"].as_str().unwrap_or("");
                id == "ddgs" || r["configured"].as_bool() == Some(true)
            })
            .filter_map(|r| r["id"].as_str().map(str::to_string))
            .collect();

        let disk = edgecrab_tools::load_web_search_config_from_disk();
        self.chain_primary_cursor = self
            .search_backend_ids
            .iter()
            .position(|id| id == &disk.primary)
            .unwrap_or(0);
        self.list_cursor = self.chain_primary_cursor;
        self.chain_fallback_order = disk
            .fallbacks
            .iter()
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> WebSetupAction {
        use crossterm::event::KeyModifiers;

        if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return WebSetupAction::None;
        }

        match self.screen {
            WebSetupScreen::Configure => self.handle_configure_key(key),
            WebSetupScreen::ConfirmAuto => self.handle_confirm_auto_key(key),
        }
    }

    fn handle_configure_key(&mut self, key: crossterm::event::KeyEvent) -> WebSetupAction {
        use crossterm::event::KeyCode;
        let n = self.search_backend_ids.len();
        match key.code {
            KeyCode::Esc => WebSetupAction::Close,
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.reload();
                self.toast = Some("Refreshed.".into());
                WebSetupAction::Redraw
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.screen = WebSetupScreen::ConfirmAuto;
                WebSetupAction::Redraw
            }
            KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
                if n > 0 {
                    if self.list_cursor > 0 {
                        self.list_cursor -= 1;
                    } else {
                        self.list_cursor = n.saturating_sub(1);
                    }
                }
                WebSetupAction::Redraw
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
                if n > 0 {
                    self.list_cursor = (self.list_cursor + 1) % n;
                }
                WebSetupAction::Redraw
            }
            KeyCode::Enter => {
                self.set_primary(self.list_cursor);
                WebSetupAction::Redraw
            }
            KeyCode::Char(' ') => {
                self.toggle_fallback(self.list_cursor);
                WebSetupAction::Redraw
            }
            _ => WebSetupAction::None,
        }
    }

    fn handle_confirm_auto_key(&mut self, key: crossterm::event::KeyEvent) -> WebSetupAction {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.screen = WebSetupScreen::Configure;
                WebSetupAction::Redraw
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                let web = edgecrab_tools::clear_web_section_overrides(&self.config_path);
                let chain = edgecrab_tools::clear_web_search_chain_in_config(&self.config_path);
                self.toast = match (web, chain) {
                    (Ok(()), Ok(())) => Some("Reset to auto — picks best configured backend.".into()),
                    (Err(e), _) | (_, Err(e)) => Some(format!("Reset failed: {e}")),
                };
                self.reload();
                self.screen = WebSetupScreen::Configure;
                WebSetupAction::Redraw
            }
            _ => WebSetupAction::None,
        }
    }

    fn set_primary(&mut self, idx: usize) {
        let Some(id) = self.search_backend_ids.get(idx).cloned() else {
            return;
        };
        self.chain_primary_cursor = idx;
        self.chain_fallback_order.retain(|x| x != &id);
        if let Err(e) = self.save_chain() {
            self.toast = Some(format!("Save failed: {e}"));
        } else {
            self.toast = Some(format!("Primary set to {id}"));
            self.reload();
        }
    }

    fn toggle_fallback(&mut self, idx: usize) {
        let Some(id) = self.search_backend_ids.get(idx).cloned() else {
            return;
        };
        if idx == self.chain_primary_cursor {
            self.toast = Some("Can't add primary as fallback — pick another.".into());
            return;
        }
        if let Some(pos) = self.chain_fallback_order.iter().position(|x| x == &id) {
            self.chain_fallback_order.remove(pos);
        } else {
            self.chain_fallback_order.push(id.clone());
        }
        if let Err(e) = self.save_chain() {
            self.toast = Some(format!("Save failed: {e}"));
        } else {
            let label = if self.chain_fallback_order.contains(&id) {
                format!("Added {id} as fallback")
            } else {
                format!("Removed {id} from fallbacks")
            };
            self.toast = Some(label);
            self.reload();
        }
    }

    fn save_chain(&self) -> anyhow::Result<()> {
        let primary = self
            .search_backend_ids
            .get(self.chain_primary_cursor)
            .cloned()
            .unwrap_or_else(|| "auto".into());
        let disk = edgecrab_tools::load_web_search_config_from_disk();
        let update = edgecrab_tools::WebSearchChainUpdate {
            primary: Some(primary),
            fallbacks: Some(self.chain_fallback_order.clone()),
            timeout_secs: Some(disk.timeout_secs.max(8)),
        };
        edgecrab_tools::clear_web_section_overrides(&self.config_path)?;
        edgecrab_tools::persist_web_search_chain_in_config(&self.config_path, &update)?;
        Ok(())
    }

    pub fn status_line(&self) -> Line<'static> {
        let report = edgecrab_tools::collect_web_diagnostics();
        let badge = if report.search_ready {
            ("✓ Ready", Color::Rgb(120, 220, 140))
        } else {
            ("✗ Setup needed", Color::Rgb(255, 130, 100))
        };
        Line::from(vec![
            Span::styled("  🔍  ", Style::default().fg(WEB_ACCENT)),
            Span::styled(
                badge.0,
                Style::default().fg(badge.1).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  ·  "),
            Span::styled(
                report.search_chain_summary,
                Style::default().fg(Color::Rgb(200, 170, 120)),
            ),
        ])
    }

    pub fn help_line() -> Line<'static> {
        Line::from(vec![
            Span::styled(" Enter ", Style::default().fg(WEB_ACCENT)),
            Span::styled("primary  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space ", Style::default().fg(WEB_ACCENT)),
            Span::styled("fallback  ", Style::default().fg(Color::DarkGray)),
            Span::styled("a ", Style::default().fg(WEB_ACCENT)),
            Span::styled("auto  ", Style::default().fg(Color::DarkGray)),
            Span::styled("r ", Style::default().fg(WEB_ACCENT)),
            Span::styled("refresh  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(WEB_ACCENT)),
            Span::styled("close", Style::default().fg(Color::DarkGray)),
        ])
    }

    pub fn selected_provider_detail(&self) -> Vec<Line<'_>> {
        let Some(id) = self.search_backend_ids.get(self.list_cursor) else {
            return vec![Line::from("No providers.")];
        };
        let row = self
            .provider_rows
            .iter()
            .find(|r| r["id"].as_str() == Some(id.as_str()));

        let mut lines = vec![
            Line::from(Span::styled(
                id.clone(),
                Style::default()
                    .fg(Color::Rgb(255, 220, 150))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if let Some(row) = row {
            lines.push(Line::from(provider_label(row)));
            if let Some(env) = row["missing_env"].as_array().filter(|a| !a.is_empty()) {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Add to ~/.edgecrab/.env:",
                    Style::default().fg(Color::Rgb(255, 160, 100)),
                )));
                for v in env {
                    if let Some(s) = v.as_str() {
                        lines.push(Line::from(format!("  {s}=...")));
                    }
                }
            } else if row["configured"].as_bool() == Some(true) {
                lines.push(Line::from(Span::styled(
                    "✓ Credentials found",
                    Style::default().fg(Color::Rgb(120, 220, 140)),
                )));
            } else if id == "ddgs" {
                lines.push(Line::from(
                    "No API key needed. May be blocked by bot checks — add a paid backend as primary.",
                ));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "How search works",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from("  1. Try primary"));
        lines.push(Line::from("  2. On failure → each fallback in order"));
        lines.push(Line::from("  3. ddgs is always last (no key)"));

        lines
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSetupAction {
    None,
    Redraw,
    Close,
}

pub fn provider_label(row: &Value) -> String {
    crate::web_setup::format_picker_label(row, row["configured"].as_bool().unwrap_or(false))
}

pub fn row_prefix(setup: &WebSetupTui, idx: usize) -> String {
    if idx == setup.chain_primary_cursor {
        return "▶ ".into();
    }
    if let Some(order) = setup
        .chain_fallback_order
        .iter()
        .position(|id| setup.search_backend_ids.get(idx) == Some(id))
    {
        return format!("{} ", order + 1);
    }
    "  ".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_fallback_skips_primary() {
        let mut setup = WebSetupTui::new(PathBuf::from("/tmp/config.yaml"));
        setup.search_backend_ids = vec!["firecrawl".into(), "ddgs".into()];
        setup.chain_primary_cursor = 0;
        setup.list_cursor = 0;
        setup.screen = WebSetupScreen::Configure;

        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        setup.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()));
        assert!(setup.toast.as_deref().is_some_and(|t| t.contains("primary")));
    }
}
