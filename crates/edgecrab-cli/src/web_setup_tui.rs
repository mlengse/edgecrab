//! `/web` — search provider priority chain editor (TUI shell over `WebChainEditor`).

use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

use crate::web_command::WEB_ACCENT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSetupScreen {
    Configure,
    ConfirmAuto,
}

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
    pub editor: edgecrab_tools::WebChainEditor,
    pub toast: Option<String>,
    config_path: PathBuf,
}

impl WebSetupTui {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            active: false,
            screen: WebSetupScreen::Configure,
            list_cursor: 0,
            editor: edgecrab_tools::WebChainEditor::load_from_disk(),
            toast: None,
            config_path,
        }
    }

    pub fn open(&mut self) {
        let _ = edgecrab_tools::migrate_legacy_search_override(&self.config_path);
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
        self.editor.reload();
        self.list_cursor = 0;
    }

    pub fn list_row_count(&self) -> usize {
        self.editor.order.len() + self.editor.available_ids().len()
    }

    pub fn row_at(&self, cursor: usize) -> Option<WebListRow> {
        if cursor < self.editor.order.len() {
            Some(WebListRow::Chain {
                index: cursor,
                id: self.editor.order[cursor].clone(),
            })
        } else {
            let ai = cursor - self.editor.order.len();
            self.editor
                .available_ids()
                .get(ai)
                .cloned()
                .map(|id| WebListRow::Available { id })
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> WebSetupAction {
        use crossterm::event::KeyModifiers;

        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
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
                self.move_chain_item(-1)
            }
            KeyCode::Char(']') | KeyCode::Char('d') | KeyCode::Char('D') => self.move_chain_item(1),
            KeyCode::Enter => self.add_selected_available(),
            KeyCode::Char('x') | KeyCode::Char('X') | KeyCode::Delete | KeyCode::Backspace => {
                self.remove_selected_chain_item()
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
                self.toast = match edgecrab_tools::reset_web_to_auto(&self.config_path) {
                    Ok(()) => {
                        Some("Auto mode — EdgeCrab picks the best configured backends.".into())
                    }
                    Err(e) => Some(format!("Reset failed: {e}")),
                };
                self.reload();
                self.screen = WebSetupScreen::Configure;
                if self
                    .toast
                    .as_ref()
                    .is_some_and(|t| !t.starts_with("Reset failed"))
                {
                    WebSetupAction::ChainSaved
                } else {
                    WebSetupAction::Redraw
                }
            }
            _ => WebSetupAction::None,
        }
    }

    fn move_chain_item(&mut self, delta: i32) -> WebSetupAction {
        let Some(WebListRow::Chain { index, .. }) = self.row_at(self.list_cursor) else {
            self.toast = Some("Select a chain row, then press [ or ] to reorder.".into());
            return WebSetupAction::Redraw;
        };
        match self.editor.move_item(index, delta) {
            Ok(new_index) => {
                self.list_cursor = new_index;
                match self.editor.persist(&self.config_path) {
                    Ok(()) => {
                        self.toast = Some(format!(
                            "Moved {} to priority #{}",
                            self.editor.order[new_index],
                            new_index + 1
                        ));
                        WebSetupAction::ChainSaved
                    }
                    Err(e) => {
                        self.toast = Some(format!("Save failed: {e}"));
                        WebSetupAction::Redraw
                    }
                }
            }
            Err(_) => WebSetupAction::Redraw,
        }
    }

    fn add_selected_available(&mut self) -> WebSetupAction {
        let Some(WebListRow::Available { id }) = self.row_at(self.list_cursor) else {
            self.toast =
                Some("Highlight a provider under “Add to chain”, then press Enter.".into());
            return WebSetupAction::Redraw;
        };
        match self.editor.add_backend(&id) {
            Ok(()) => match self.editor.persist(&self.config_path) {
                Ok(()) => {
                    self.toast = Some(format!("Added {id} as #{}", self.editor.order.len()));
                    self.reload();
                    if let Some(pos) = self.editor.order.iter().position(|x| x == &id) {
                        self.list_cursor = pos;
                    }
                    WebSetupAction::ChainSaved
                }
                Err(e) => {
                    let _ = self
                        .editor
                        .remove_at(self.editor.order.len().saturating_sub(1));
                    self.toast = Some(format!("Save failed: {e}"));
                    WebSetupAction::Redraw
                }
            },
            Err(e) => {
                self.toast = Some(e.message().into());
                WebSetupAction::Redraw
            }
        }
    }

    fn remove_selected_chain_item(&mut self) -> WebSetupAction {
        let Some(WebListRow::Chain { index, .. }) = self.row_at(self.list_cursor) else {
            self.toast = Some("Select a chain row to remove (x).".into());
            return WebSetupAction::Redraw;
        };
        match self.editor.remove_at(index) {
            Ok(id) => {
                self.list_cursor = self
                    .list_cursor
                    .min(self.list_row_count().saturating_sub(1));
                match self.editor.persist(&self.config_path) {
                    Ok(()) => {
                        self.toast = Some(format!("Removed {id} from chain"));
                        WebSetupAction::ChainSaved
                    }
                    Err(e) => {
                        self.reload();
                        self.toast = Some(format!("Save failed: {e}"));
                        WebSetupAction::Redraw
                    }
                }
            }
            Err(e) => {
                self.toast = Some(e.message().into());
                WebSetupAction::Redraw
            }
        }
    }

    pub fn override_warning_line(&self) -> Option<Line<'static>> {
        edgecrab_tools::search_override_warning().map(|w| {
            Line::from(Span::styled(
                format!("  {w}"),
                Style::default().fg(Color::Rgb(255, 200, 100)),
            ))
        })
    }

    pub fn chain_summary_line(&self) -> Line<'static> {
        let mode = if self.editor.is_auto {
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
                self.editor.summary_arrow(),
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
        let chain_title = if self.editor.is_auto {
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

        for (i, id) in self.editor.order.iter().enumerate() {
            let is_cursor = self.row_at(self.list_cursor)
                == Some(WebListRow::Chain {
                    index: i,
                    id: id.clone(),
                });
            items.push(chain_list_item(
                self,
                id,
                i,
                is_cursor,
                accent,
                self.editor.is_auto,
            ));
        }

        let available = self.editor.available_ids();
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

        match row {
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
                lines.extend(detail_lines(&self.editor, &id));
                lines
            }
            WebListRow::Available { id } => {
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
                lines.extend(detail_lines(&self.editor, &id));
                lines
            }
        }
    }
}

fn detail_lines(editor: &edgecrab_tools::WebChainEditor, id: &str) -> Vec<Line<'static>> {
    let row = editor.catalog.row_for(id);
    let mut lines = vec![Line::from("")];
    for text in edgecrab_tools::provider_detail_lines(row, id) {
        if text.is_empty() {
            lines.push(Line::from(""));
        } else if text.starts_with("Add missing") {
            lines.push(Line::from(Span::styled(
                text,
                Style::default().fg(Color::Rgb(255, 160, 100)),
            )));
        } else if text.starts_with('✓') {
            lines.push(Line::from(Span::styled(
                text,
                Style::default().fg(Color::Rgb(120, 220, 140)),
            )));
        } else {
            lines.push(Line::from(text));
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
    let label = setup.editor.catalog.picker_label(id);
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
    let label = setup.editor.catalog.picker_label(id);
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
        Span::styled(
            " › ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSetupAction {
    None,
    Redraw,
    Close,
    /// Chain persisted to config.yaml — caller should sync agent snapshot.
    ChainSaved,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_setup() -> WebSetupTui {
        let mut setup = WebSetupTui::new(PathBuf::from("/tmp/config.yaml"));
        setup.editor.catalog.chain_eligible_ids = vec![
            "searxng".into(),
            "brave".into(),
            "firecrawl".into(),
            "ddgs".into(),
        ];
        setup.editor.order = vec!["searxng".into(), "ddgs".into()];
        setup.editor.is_auto = false;
        setup.list_cursor = 1;
        setup.screen = WebSetupScreen::Configure;
        setup
    }

    #[test]
    fn move_chain_item_swaps_order() {
        let mut setup = test_setup();
        setup.move_chain_item(-1);
        assert_eq!(setup.editor.order, vec!["ddgs", "searxng"]);
        assert_eq!(setup.list_cursor, 0);
    }

    #[test]
    fn available_ids_excludes_chain_members() {
        let setup = test_setup();
        let available = setup.editor.available_ids();
        assert!(!available.contains(&"searxng".into()));
        assert!(available.contains(&"brave".into()));
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
        assert!(matches!(
            setup.row_at(2),
            Some(WebListRow::Available { .. })
        ));
    }
}
