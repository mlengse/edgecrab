//! `/web` — search provider priority chain editor.
//!
//! One ordered list: try #1 first, then #2, and so on. No separate primary/fallback keys.

use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;
use serde_json::Value;

use crate::web_command::WEB_ACCENT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSetupScreen {
    Configure,
    ConfirmAuto,
}

/// A row in the chain / available-provider list (no separator rows — cursor-safe).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebListRow {
    Chain { index: usize, id: String },
    Available { id: String },
}

#[derive(Debug, Clone)]
pub struct WebSetupTui {
    pub active: bool,
    pub screen: WebSetupScreen,
    pub list_cursor: usize,
    pub provider_rows: Vec<Value>,
    /// Backends that can be placed in the chain (configured search providers + ddgs).
    search_backend_ids: Vec<String>,
    /// Priority order: index 0 = tried first.
    pub chain_order: Vec<String>,
    /// True when config has no explicit primary (EdgeCrab picks the chain automatically).
    pub is_auto: bool,
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
            chain_order: Vec::new(),
            is_auto: true,
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
        self.is_auto = disk.primary.trim().is_empty();

        self.chain_order = if self.is_auto {
            edgecrab_tools::ResolvedChain::resolve(&disk, None)
                .ok()
                .map(|r| r.names)
                .filter(|names| !names.is_empty())
                .unwrap_or_else(|| vec!["ddgs".into()])
        } else {
            let mut chain = vec![disk.primary.trim().to_ascii_lowercase()];
            for fb in &disk.fallbacks {
                let fb = fb.trim().to_ascii_lowercase();
                if !fb.is_empty() && !chain.contains(&fb) {
                    chain.push(fb);
                }
            }
            chain
        };

        self.list_cursor = 0;
    }

    pub fn list_row_count(&self) -> usize {
        self.chain_order.len() + self.available_backend_ids().len()
    }

    pub fn row_at(&self, cursor: usize) -> Option<WebListRow> {
        if cursor < self.chain_order.len() {
            Some(WebListRow::Chain {
                index: cursor,
                id: self.chain_order[cursor].clone(),
            })
        } else {
            let ai = cursor - self.chain_order.len();
            self.available_backend_ids()
                .get(ai)
                .cloned()
                .map(|id| WebListRow::Available { id })
        }
    }

    pub fn available_backend_ids(&self) -> Vec<String> {
        self.search_backend_ids
            .iter()
            .filter(|id| !self.chain_order.contains(id))
            .cloned()
            .collect()
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
        let n = self.list_row_count();
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
                    self.list_cursor = if self.list_cursor == 0 {
                        n - 1
                    } else {
                        self.list_cursor - 1
                    };
                }
                WebSetupAction::Redraw
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
                if n > 0 {
                    self.list_cursor = (self.list_cursor + 1) % n;
                }
                WebSetupAction::Redraw
            }
            KeyCode::Char('[') | KeyCode::Char('u') | KeyCode::Char('U') => {
                self.move_chain_item(-1);
                WebSetupAction::Redraw
            }
            KeyCode::Char(']') | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.move_chain_item(1);
                WebSetupAction::Redraw
            }
            KeyCode::Enter => {
                self.add_selected_available();
                WebSetupAction::Redraw
            }
            KeyCode::Char('x') | KeyCode::Char('X') | KeyCode::Delete | KeyCode::Backspace => {
                self.remove_selected_chain_item();
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
                    (Ok(()), Ok(())) => {
                        Some("Auto mode — EdgeCrab picks the best configured backends.".into())
                    }
                    (Err(e), _) | (_, Err(e)) => Some(format!("Reset failed: {e}")),
                };
                self.reload();
                self.screen = WebSetupScreen::Configure;
                WebSetupAction::Redraw
            }
            _ => WebSetupAction::None,
        }
    }

    fn move_chain_item(&mut self, delta: i32) {
        let Some(WebListRow::Chain { index, .. }) = self.row_at(self.list_cursor) else {
            self.toast = Some("Select a chain row, then press [ or ] to reorder.".into());
            return;
        };
        let new_index = index as i32 + delta;
        if new_index < 0 || new_index as usize >= self.chain_order.len() {
            return;
        }
        let new_index = new_index as usize;
        self.chain_order.swap(index, new_index);
        self.list_cursor = new_index;
        self.is_auto = false;
        match self.save_chain() {
            Ok(()) => {
                self.toast = Some(format!(
                    "Moved {} to priority #{}",
                    self.chain_order[new_index],
                    new_index + 1
                ));
            }
            Err(e) => self.toast = Some(format!("Save failed: {e}")),
        }
    }

    fn add_selected_available(&mut self) {
        let Some(WebListRow::Available { id }) = self.row_at(self.list_cursor) else {
            self.toast =
                Some("Highlight a provider under “Add to chain”, then press Enter.".into());
            return;
        };
        if self.chain_order.contains(&id) {
            return;
        }
        self.chain_order.push(id.clone());
        self.is_auto = false;
        match self.save_chain() {
            Ok(()) => {
                self.toast = Some(format!("Added {id} as #{ }", self.chain_order.len()));
                self.reload();
                if let Some(pos) = self.chain_order.iter().position(|x| x == &id) {
                    self.list_cursor = pos;
                }
            }
            Err(e) => {
                self.chain_order.retain(|x| x != &id);
                self.toast = Some(format!("Save failed: {e}"));
            }
        }
    }

    fn remove_selected_chain_item(&mut self) {
        let Some(WebListRow::Chain { index, id }) = self.row_at(self.list_cursor) else {
            self.toast = Some("Select a chain row to remove (x).".into());
            return;
        };
        if self.chain_order.len() <= 1 {
            self.toast = Some("Keep at least one provider — use a for auto reset.".into());
            return;
        }
        self.chain_order.remove(index);
        self.is_auto = false;
        self.list_cursor = self.list_cursor.min(self.list_row_count().saturating_sub(1));
        match self.save_chain() {
            Ok(()) => self.toast = Some(format!("Removed {id} from chain")),
            Err(e) => {
                self.reload();
                self.toast = Some(format!("Save failed: {e}"));
            }
        }
    }

    fn save_chain(&self) -> anyhow::Result<()> {
        let primary = self
            .chain_order
            .first()
            .cloned()
            .unwrap_or_else(|| "ddgs".into());
        let fallbacks = self.chain_order.iter().skip(1).cloned().collect();
        let disk = edgecrab_tools::load_web_search_config_from_disk();
        let update = edgecrab_tools::WebSearchChainUpdate {
            primary: Some(primary),
            fallbacks: Some(fallbacks),
            timeout_secs: Some(disk.timeout_secs.max(8)),
        };
        edgecrab_tools::clear_web_section_overrides(&self.config_path)?;
        edgecrab_tools::persist_web_search_chain_in_config(&self.config_path, &update)?;
        Ok(())
    }

    pub fn chain_summary_line(&self) -> Line<'static> {
        let chain_text = if self.chain_order.is_empty() {
            "ddgs".to_string()
        } else {
            self.chain_order.join(" → ")
        };
        let mode = if self.is_auto {
            Span::styled(
                "auto · ",
                Style::default()
                    .fg(Color::Rgb(160, 200, 255))
                    .add_modifier(Modifier::ITALIC),
            )
        } else {
            Span::styled(
                "custom · ",
                Style::default()
                    .fg(Color::Rgb(255, 200, 120))
                    .add_modifier(Modifier::BOLD),
            )
        };
        Line::from(vec![
            Span::raw("  Priority: "),
            mode,
            Span::styled(
                chain_text,
                Style::default().fg(Color::Rgb(200, 170, 120)),
            ),
            Span::raw("  (try left → right)"),
        ])
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
            Span::styled(" ↑↓ ", Style::default().fg(WEB_ACCENT)),
            Span::styled("nav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[ ] ", Style::default().fg(WEB_ACCENT)),
            Span::styled("reorder  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter ", Style::default().fg(WEB_ACCENT)),
            Span::styled("add  ", Style::default().fg(Color::DarkGray)),
            Span::styled("x ", Style::default().fg(WEB_ACCENT)),
            Span::styled("remove  ", Style::default().fg(Color::DarkGray)),
            Span::styled("a ", Style::default().fg(WEB_ACCENT)),
            Span::styled("auto  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc ", Style::default().fg(WEB_ACCENT)),
            Span::styled("close", Style::default().fg(Color::DarkGray)),
        ])
    }

    pub fn build_list_items(&self, accent: Color) -> Vec<ListItem<'static>> {
        let mut items = Vec::new();

        let chain_title = if self.is_auto {
            " Chain (auto preview — edit to customize) "
        } else {
            " Your chain — tried top to bottom "
        };
        items.push(ListItem::new(Line::from(Span::styled(
            chain_title,
            Style::default()
                .fg(Color::Rgb(180, 150, 100))
                .add_modifier(Modifier::BOLD),
        ))));

        for (i, id) in self.chain_order.iter().enumerate() {
            let is_cursor = self.row_at(self.list_cursor) == Some(WebListRow::Chain {
                index: i,
                id: id.clone(),
            });
            items.push(chain_list_item(self, id, i, is_cursor, accent, self.is_auto));
        }

        let available = self.available_backend_ids();
        if !available.is_empty() {
            items.push(ListItem::new(Line::from("")));
            items.push(ListItem::new(Line::from(Span::styled(
                " Add to chain (Enter) ",
                Style::default()
                    .fg(Color::Rgb(140, 180, 220))
                    .add_modifier(Modifier::BOLD),
            ))));
            for id in available {
                let is_cursor =
                    self.row_at(self.list_cursor) == Some(WebListRow::Available { id: id.clone() });
                items.push(available_list_item(self, &id, is_cursor, accent));
            }
        }

        items
    }

    pub fn selected_provider_detail(&self) -> Vec<Line<'static>> {
        let Some(row) = self.row_at(self.list_cursor) else {
            return vec![Line::from("No providers.")];
        };

        let id = match &row {
            WebListRow::Chain { id, index } => {
                let mut lines = vec![
                    Line::from(Span::styled(
                        format!("#{} in chain", index + 1),
                        Style::default()
                            .fg(Color::Rgb(255, 220, 150))
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        id.clone(),
                        Style::default().fg(Color::Rgb(255, 220, 150)),
                    )),
                    Line::from(""),
                    Line::from("[ move up   ] move down   x remove"),
                ];
                lines.extend(provider_detail_lines(self, id));
                return lines;
            }
            WebListRow::Available { id } => id,
        };

        let mut lines = vec![
            Line::from(Span::styled(
                "Add to chain",
                Style::default()
                    .fg(Color::Rgb(140, 180, 220))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                id.clone(),
                Style::default().fg(Color::Rgb(255, 220, 150)),
            )),
            Line::from(""),
            Line::from("Enter — append as last fallback"),
        ];
        lines.extend(provider_detail_lines(self, &id));
        lines
    }
}

fn provider_detail_lines(setup: &WebSetupTui, id: &str) -> Vec<Line<'static>> {
    let row = setup
        .provider_rows
        .iter()
        .find(|r| r["id"].as_str() == Some(id));

    let mut lines = vec![Line::from("")];
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
                "No API key. Often last resort — may hit bot checks on some networks.",
            ));
        }
    }
    lines
}

fn chain_list_item(
    setup: &WebSetupTui,
    id: &str,
    index: usize,
    is_cursor: bool,
    accent: Color,
    dimmed: bool,
) -> ListItem<'static> {
    let row = setup
        .provider_rows
        .iter()
        .find(|r| r["id"].as_str() == Some(id));
    let label = row.map(provider_label).unwrap_or_else(|| id.to_string());
    let fg = if is_cursor {
        Color::White
    } else if dimmed {
        Color::Rgb(180, 160, 130)
    } else {
        Color::Rgb(230, 200, 150)
    };
    ListItem::new(Line::from(vec![
        selector_marker(is_cursor, accent),
        Span::styled(
            format!(" {:>2}. {label}", index + 1),
            Style::default().fg(fg),
        ),
    ]))
}

fn available_list_item(
    setup: &WebSetupTui,
    id: &str,
    is_cursor: bool,
    accent: Color,
) -> ListItem<'static> {
    let row = setup
        .provider_rows
        .iter()
        .find(|r| r["id"].as_str() == Some(id));
    let label = row.map(provider_label).unwrap_or_else(|| id.to_string());
    ListItem::new(Line::from(vec![
        selector_marker(is_cursor, accent),
        Span::styled(
            format!("  + {label}"),
            Style::default().fg(if is_cursor {
                Color::White
            } else {
                Color::Rgb(190, 210, 230)
            }),
        ),
    ]))
}

fn selector_marker(is_cursor: bool, accent: Color) -> Span<'static> {
    if is_cursor {
        Span::styled(" › ", Style::default().fg(accent).add_modifier(Modifier::BOLD))
    } else {
        Span::raw("   ")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_setup() -> WebSetupTui {
        let mut setup = WebSetupTui::new(PathBuf::from("/tmp/config.yaml"));
        setup.search_backend_ids = vec![
            "searxng".into(),
            "brave".into(),
            "firecrawl".into(),
            "ddgs".into(),
        ];
        setup.chain_order = vec!["searxng".into(), "ddgs".into()];
        setup.is_auto = false;
        setup.list_cursor = 1;
        setup.screen = WebSetupScreen::Configure;
        setup
    }

    #[test]
    fn move_chain_item_swaps_order() {
        let mut setup = test_setup();
        setup.move_chain_item(-1);
        assert_eq!(setup.chain_order, vec!["ddgs", "searxng"]);
        assert_eq!(setup.list_cursor, 0);
    }

    #[test]
    fn available_ids_excludes_chain_members() {
        let setup = test_setup();
        assert!(!setup.available_backend_ids().contains(&"searxng".into()));
        assert!(setup.available_backend_ids().contains(&"brave".into()));
    }

    #[test]
    fn row_at_maps_chain_and_available() {
        let setup = test_setup();
        assert_eq!(
            setup.row_at(0),
            Some(WebListRow::Chain {
                index: 0,
                id: "searxng".into()
            })
        );
        assert!(matches!(setup.row_at(2), Some(WebListRow::Available { .. })));
    }
}
