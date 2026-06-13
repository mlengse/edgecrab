//! In-TUI xAI Grok OAuth — start in overlay; finish via clipboard or terminal readline (reliable).

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::auth_cmd;
use crate::proxy_hub::PROXY_ACCENT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokAuthScreen {
    /// Open x.ai and save PKCE session (~30 min).
    Start,
    /// Submit code (clipboard or terminal readline — no in-TUI text box).
    Finish,
    /// Brief success before close.
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokAuthAction {
    None,
    Close,
    /// Run `start_xai_oauth_login`.
    RunStart,
    /// Load clipboard into `pending_code`, then user presses Enter again.
    LoadClipboard,
    /// Submit: clipboard → pending → suspended readline (in that order).
    RunFinish,
    /// Open authorize URL in the system browser.
    OpenBrowser,
}

pub struct GrokAuthTui {
    pub active: bool,
    pub screen: GrokAuthScreen,
    pub busy: bool,
    pub error: Option<String>,
    pub authorize_url: Option<String>,
    pub pending_path: Option<PathBuf>,
    pub success_message: Option<String>,
    /// Normalized code ready to exchange (set by `p` or clipboard on Enter).
    pub pending_code: Option<String>,
    pub(crate) no_browser: bool,
}

impl GrokAuthTui {
    pub fn new() -> Self {
        Self {
            active: false,
            screen: GrokAuthScreen::Start,
            busy: false,
            error: None,
            authorize_url: None,
            pending_path: None,
            success_message: None,
            pending_code: None,
            no_browser: false,
        }
    }

    pub fn open(&mut self, screen: GrokAuthScreen) {
        self.busy = false;
        self.error = None;
        self.success_message = None;
        self.pending_code = None;
        self.no_browser = false;

        if let Some((url, path)) = auth_cmd::grok_load_valid_pending()
            && matches!(screen, GrokAuthScreen::Finish | GrokAuthScreen::Start)
        {
            self.screen = GrokAuthScreen::Finish;
            self.pending_path = Some(path);
            self.authorize_url = Some(url);
        } else {
            self.screen = screen;
            self.authorize_url = None;
            self.pending_path = None;
        }

        self.active = true;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.busy = false;
        self.pending_code = None;
    }

    pub fn set_start_result(&mut self, authorize_url: String, pending_path: PathBuf) {
        self.authorize_url = Some(authorize_url);
        self.pending_path = Some(pending_path);
        self.screen = GrokAuthScreen::Finish;
        self.busy = false;
        self.error = None;
        self.pending_code = None;
    }

    pub fn set_finish_success(&mut self, message: String) {
        self.success_message = Some(message);
        self.screen = GrokAuthScreen::Done;
        self.busy = false;
        self.error = None;
        self.pending_code = None;
    }

    pub fn set_error(&mut self, message: String) {
        self.error = Some(message);
        self.busy = false;
    }

    pub fn set_pending_code(&mut self, code: String) {
        self.pending_code = Some(code);
        self.error = None;
    }

    pub fn code_status_line(&self) -> Option<Line<'static>> {
        self.pending_code.as_ref().map(|code| {
            Line::from(Span::styled(
                format!("Code ready: {}", auth_cmd::mask_grok_code(code)),
                Style::default().fg(Color::Rgb(140, 220, 160)),
            ))
        })
    }

    pub fn title(&self) -> &'static str {
        match self.screen {
            GrokAuthScreen::Start => " Grok sign-in (step 1) ",
            GrokAuthScreen::Finish => " Grok sign-in (step 2) ",
            GrokAuthScreen::Done => " Grok sign-in — done ",
        }
    }

    pub fn body_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        if self.busy {
            lines.push(Line::from(Span::styled(
                "Saving tokens…",
                Style::default()
                    .fg(Color::Rgb(255, 200, 120))
                    .add_modifier(Modifier::ITALIC),
            )));
            lines.push(Line::from(""));
        }

        if let Some(ref err) = self.error {
            lines.push(Line::from(Span::styled(
                format!("Error: {err}"),
                Style::default().fg(Color::Rgb(239, 83, 80)),
            )));
            lines.push(Line::from(""));
        }

        match self.screen {
            GrokAuthScreen::Start => {
                lines.extend(start_instruction_lines());
            }
            GrokAuthScreen::Finish => {
                lines.extend(finish_instruction_lines());
                if let Some(status) = self.code_status_line() {
                    lines.push(Line::from(""));
                    lines.push(status);
                }
            }
            GrokAuthScreen::Done => {
                let msg = self
                    .success_message
                    .as_deref()
                    .unwrap_or("Grok OAuth saved.");
                for part in msg.lines() {
                    lines.push(Line::from(Span::styled(
                        part.to_string(),
                        Style::default().fg(Color::Rgb(140, 220, 160)),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(Line::from("Press Enter to close."));
            }
        }

        lines
    }

    pub fn help_line(&self) -> Line<'static> {
        if self.busy {
            return Line::from(Span::styled(
                " Please wait… ",
                Style::default().fg(Color::DarkGray),
            ));
        }
        let hint = match self.screen {
            GrokAuthScreen::Start => " Enter — open x.ai  ·  Esc — cancel ",
            GrokAuthScreen::Finish => {
                " Enter — submit  ·  p — load clipboard  ·  o — reopen URL  ·  Esc — cancel "
            }
            GrokAuthScreen::Done => " Enter / Esc — close ",
        };
        Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray)))
    }

    pub fn is_submit_key(key: &KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (_, KeyCode::Enter) => true,
            (KeyModifiers::CONTROL, KeyCode::Char('j')) => true,
            (KeyModifiers::CONTROL, KeyCode::Char('m')) => true,
            (m, KeyCode::Char('\r')) if !m.intersects(KeyModifiers::SHIFT) => true,
            (m, KeyCode::Char('\n')) if !m.intersects(KeyModifiers::SHIFT) => true,
            _ => false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> GrokAuthAction {
        if self.busy {
            return GrokAuthAction::None;
        }

        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return GrokAuthAction::None;
        }

        match self.screen {
            GrokAuthScreen::Done => match key.code {
                KeyCode::Esc | KeyCode::Enter => GrokAuthAction::Close,
                _ => GrokAuthAction::None,
            },
            GrokAuthScreen::Start => match key.code {
                KeyCode::Esc => GrokAuthAction::Close,
                KeyCode::Enter => GrokAuthAction::RunStart,
                KeyCode::Char('o') | KeyCode::Char('O') if self.authorize_url.is_some() => {
                    GrokAuthAction::OpenBrowser
                }
                _ => GrokAuthAction::None,
            },
            GrokAuthScreen::Finish => {
                if Self::is_submit_key(&key) {
                    return GrokAuthAction::RunFinish;
                }
                match key.code {
                    KeyCode::Esc => GrokAuthAction::Close,
                    KeyCode::Char('p') | KeyCode::Char('P') => GrokAuthAction::LoadClipboard,
                    KeyCode::Char('o') | KeyCode::Char('O') => GrokAuthAction::OpenBrowser,
                    _ => GrokAuthAction::None,
                }
            }
        }
    }
}

fn start_instruction_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "SuperGrok / X Premium+",
            Style::default()
                .fg(PROXY_ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("1. Press Enter — opens x.ai in your browser."),
        Line::from("2. Sign in. If you see \"Could not establish connection\","),
        Line::from("   copy the code shown on that page (not the URL)."),
        Line::from(""),
        Line::from(Span::styled(
            "Step 2 uses your clipboard or a plain terminal prompt — not an in-TUI text field.",
            Style::default().fg(Color::Rgb(255, 200, 120)),
        )),
    ]
}

fn finish_instruction_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Step 2 — Submit authorization code",
            Style::default()
                .fg(PROXY_ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("1. Copy the code from x.ai (the long token, not the URL)."),
        Line::from("2. Press p to load from clipboard, or press Enter to submit."),
        Line::from(
            "3. Enter: clipboard → terminal prompt; if the session expired, opens x.ai again.",
        ),
        Line::from(""),
        Line::from(Span::styled(
            "The terminal paste prompt always works (same as Copilot /login).",
            Style::default().fg(Color::DarkGray),
        )),
    ]
}
