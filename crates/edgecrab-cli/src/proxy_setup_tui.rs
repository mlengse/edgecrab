//! `/proxy` — in-TUI OpenAI-compat proxy setup (Grok/xAI, Nous presets).

use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

use edgecrab_proxy::{ALL_RECIPES, BuiltinRecipe, probe_oauth_auth, AuthProbe};

use crate::proxy_cmd::ProxySession;
use crate::proxy_hub::{self, PROXY_ACCENT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySetupScreen {
    PickPreset,
    ConfirmEnable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySetupAction {
    None,
    Redraw,
    Close,
    /// Config saved — show toast only.
    ConfigSaved,
}

pub struct ProxySetupTui {
    pub active: bool,
    pub screen: ProxySetupScreen,
    pub list_cursor: usize,
    pub toast: Option<String>,
    config_path: PathBuf,
    pending_recipe: Option<&'static BuiltinRecipe>,
}

impl ProxySetupTui {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            active: false,
            screen: ProxySetupScreen::PickPreset,
            list_cursor: 0,
            toast: None,
            config_path,
            pending_recipe: None,
        }
    }

    pub fn open(&mut self) {
        self.screen = ProxySetupScreen::PickPreset;
        self.list_cursor = 0;
        self.toast = None;
        self.pending_recipe = None;
        self.active = true;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.screen = ProxySetupScreen::PickPreset;
        self.pending_recipe = None;
    }

    fn session(&self) -> Option<ProxySession> {
        ProxySession::load().ok()
    }

    pub fn status_line(&self) -> String {
        let Ok(session) = ProxySession::load() else {
            return "Could not load config.".into();
        };
        let cfg = session.proxy();
        format!(
            "Listen {} · token {} · {} alias(es) · {} upstream(s)",
            proxy_hub::listen_url(cfg),
            if session.token_present() { "ok" } else { "missing" },
            cfg.model_aliases.len(),
            cfg.forward_upstreams.len()
        )
    }

    pub fn build_list_items(&self, accent: Color) -> Vec<ListItem<'static>> {
        let session = self.session();
        ALL_RECIPES
            .iter()
            .enumerate()
            .map(|(i, recipe)| {
                let probe = probe_oauth_auth(recipe);
                let auth_icon = match probe {
                    AuthProbe::Ready => "✓",
                    AuthProbe::ReloginRequired => "✗",
                    _ => "○",
                };
                let enabled = session
                    .as_ref()
                    .map(|s| proxy_hub::recipe_enabled_in_config(s.proxy(), recipe))
                    .unwrap_or(false);
                let en = if enabled { "ON " } else { "   " };
                let cursor = if i == self.list_cursor { "▸ " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{cursor}{en}{auth_icon} "),
                        Style::default().fg(if i == self.list_cursor {
                            accent
                        } else {
                            Color::Gray
                        }),
                    ),
                    Span::styled(
                        recipe.display_name.to_string(),
                        Style::default()
                            .fg(if i == self.list_cursor {
                                accent
                            } else {
                                Color::White
                            })
                            .add_modifier(if i == self.list_cursor {
                                Modifier::BOLD
                            } else {
                                Modifier::empty()
                            }),
                    ),
                    Span::styled(
                        format!("  alias:{}  key:{}", recipe.default_alias, recipe.key),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect()
    }

    pub fn selected_recipe(&self) -> &'static BuiltinRecipe {
        ALL_RECIPES
            .get(self.list_cursor)
            .unwrap_or(&edgecrab_proxy::RECIPE_XAI)
    }

    pub fn detail_lines(&self) -> Vec<Line<'static>> {
        let recipe = self.selected_recipe();
        let session = self.session();
        let enabled = session
            .as_ref()
            .map(|s| proxy_hub::recipe_enabled_in_config(s.proxy(), recipe))
            .unwrap_or(false);

        let mut lines = vec![
            Line::from(Span::styled(
                recipe.display_name,
                Style::default()
                    .fg(PROXY_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(format!(
                "Model alias \"{}\" → forward:{}",
                recipe.default_alias, recipe.key
            )),
            Line::from(proxy_hub::format_recipe_auth_line(recipe)),
            Line::from(format!("Hermes: {}", recipe.hermes_auth_cmd)),
            Line::from(""),
            Line::from(if enabled {
                "Config: preset enabled in config.yaml"
            } else {
                "Config: not enabled — Enter to enable + create token"
            }),
            Line::from(""),
            Line::from("After enable, start in a terminal:"),
            Line::from(format!(
                "  edgecrab proxy start --provider {}",
                recipe.key
            )),
            Line::from(format!(
                "  Clients: OPENAI_API_BASE={}",
                session
                    .as_ref()
                    .map(|s| proxy_hub::listen_url(s.proxy()))
                    .unwrap_or_else(|| "http://127.0.0.1:11434/v1".to_string())
            )),
            Line::from(format!("  Model: {}", recipe.default_alias)),
        ];
        if let Some(ref t) = self.toast {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                t.clone(),
                Style::default().fg(Color::Rgb(140, 220, 160)),
            )));
        }
        lines
    }

    pub fn confirm_lines(&self) -> Vec<Line<'static>> {
        let recipe = self.pending_recipe.unwrap_or_else(|| self.selected_recipe());
        vec![
            Line::from(Span::styled(
                format!("Enable {}?", recipe.display_name),
                Style::default().fg(PROXY_ACCENT),
            )),
            Line::from(format!(
                "Writes upstream `{}` and alias `{}` to",
                recipe.key, recipe.default_alias
            )),
            Line::from(self.config_path.display().to_string()),
            Line::from("Creates proxy token if missing."),
            Line::from(""),
            Line::from("y / Enter — confirm   n / Esc — cancel"),
        ]
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ProxySetupAction {
        use crossterm::event::KeyModifiers;

        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return ProxySetupAction::None;
        }

        match self.screen {
            ProxySetupScreen::PickPreset => self.handle_pick_key(key),
            ProxySetupScreen::ConfirmEnable => self.handle_confirm_key(key),
        }
    }

    fn handle_pick_key(&mut self, key: crossterm::event::KeyEvent) -> ProxySetupAction {
        use crossterm::event::KeyCode;
        let n = ALL_RECIPES.len();
        match key.code {
            KeyCode::Esc => ProxySetupAction::Close,
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.toast = Some("Refreshed.".into());
                ProxySetupAction::Redraw
            }
            KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
                if n > 0 {
                    self.list_cursor = if self.list_cursor == 0 {
                        n - 1
                    } else {
                        self.list_cursor - 1
                    };
                }
                ProxySetupAction::Redraw
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
                if n > 0 {
                    self.list_cursor = (self.list_cursor + 1) % n;
                }
                ProxySetupAction::Redraw
            }
            KeyCode::Enter | KeyCode::Char('e') | KeyCode::Char('E') => {
                self.pending_recipe = Some(self.selected_recipe());
                self.screen = ProxySetupScreen::ConfirmEnable;
                ProxySetupAction::Redraw
            }
            _ => ProxySetupAction::None,
        }
    }

    fn handle_confirm_key(&mut self, key: crossterm::event::KeyEvent) -> ProxySetupAction {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.screen = ProxySetupScreen::PickPreset;
                self.pending_recipe = None;
                ProxySetupAction::Redraw
            }
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                let recipe = self.pending_recipe.unwrap_or_else(|| self.selected_recipe());
                match ProxySession::load() {
                    Ok(mut session) => match proxy_hub::enable_preset(&mut session, recipe) {
                        Ok(msg) => {
                            self.toast = Some(msg.lines().next().unwrap_or("Enabled.").into());
                            self.screen = ProxySetupScreen::PickPreset;
                            self.pending_recipe = None;
                            ProxySetupAction::ConfigSaved
                        }
                        Err(e) => {
                            self.toast = Some(format!("Error: {e}"));
                            self.screen = ProxySetupScreen::PickPreset;
                            ProxySetupAction::Redraw
                        }
                    },
                    Err(e) => {
                        self.toast = Some(format!("Load failed: {e}"));
                        ProxySetupAction::Redraw
                    }
                }
            }
            _ => ProxySetupAction::None,
        }
    }

    pub fn help_line() -> Line<'static> {
        Line::from(Span::styled(
            " ↑↓ move · Enter enable · r refresh · Esc close ",
            Style::default().fg(Color::DarkGray),
        ))
    }

    pub fn confirm_help_line() -> Line<'static> {
        Line::from(Span::styled(
            " y/Enter confirm · n/Esc cancel ",
            Style::default().fg(Color::DarkGray),
        ))
    }
}
