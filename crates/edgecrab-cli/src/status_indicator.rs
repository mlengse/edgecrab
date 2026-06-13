//! Busy status-bar indicator styles — Hermes `/indicator` + `FaceTicker` parity.

use crate::status_chrome::{TerminalGlyphProfile, compact_spinner_frame};
use crate::theme::Theme;

/// Hermes `IndicatorStyle` — persisted as `display.status_indicator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StatusIndicatorStyle {
    #[default]
    Kaomoji,
    Emoji,
    Unicode,
    Ascii,
}

impl StatusIndicatorStyle {
    pub const ALL: [&'static str; 4] = ["kaomoji", "emoji", "unicode", "ascii"];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "kaomoji" | "face" | "faces" => Some(Self::Kaomoji),
            "emoji" | "emojis" => Some(Self::Emoji),
            "unicode" | "braille" => Some(Self::Unicode),
            "ascii" => Some(Self::Ascii),
            _ => None,
        }
    }

    pub fn from_config(raw: &str) -> Self {
        Self::parse(raw).unwrap_or_default()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kaomoji => "kaomoji",
            Self::Emoji => "emoji",
            Self::Unicode => "unicode",
            Self::Ascii => "ascii",
        }
    }
}

const EMOJI_FRAMES: &[&str] = &["⚕ ", "🌀", "🤔", "✨", "🍵", "🔮"];
const ASCII_FRAMES: &[&str] = &["|", "/", "-", "\\"];

/// Leading glyph (+ optional face) for status-bar busy states.
pub struct IndicatorFrame {
    pub leading: String,
    pub face: String,
    pub show_verb: bool,
}

fn phase_face(faces: &[String], idx: usize) -> &str {
    if faces.is_empty() {
        ""
    } else {
        faces[idx % faces.len()].as_str()
    }
}

/// Render the active indicator frame (Hermes `renderIndicator`).
pub fn render_indicator_frame(
    style: StatusIndicatorStyle,
    theme: &Theme,
    glyphs: TerminalGlyphProfile,
    spinner_frame: usize,
    face_idx: usize,
) -> IndicatorFrame {
    match style {
        StatusIndicatorStyle::Kaomoji => IndicatorFrame {
            leading: compact_spinner_frame(spinner_frame, glyphs).to_string(),
            face: phase_face(&theme.kaomoji_waiting, face_idx).to_string(),
            show_verb: true,
        },
        StatusIndicatorStyle::Emoji => IndicatorFrame {
            leading: String::new(),
            face: EMOJI_FRAMES[face_idx % EMOJI_FRAMES.len()].to_string(),
            show_verb: true,
        },
        StatusIndicatorStyle::Unicode => IndicatorFrame {
            leading: compact_spinner_frame(spinner_frame, TerminalGlyphProfile::Unicode)
                .to_string(),
            face: String::new(),
            show_verb: false,
        },
        StatusIndicatorStyle::Ascii => IndicatorFrame {
            leading: ASCII_FRAMES[spinner_frame % ASCII_FRAMES.len()].to_string(),
            face: String::new(),
            show_verb: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_indicator_styles() {
        assert_eq!(
            StatusIndicatorStyle::parse("kaomoji"),
            Some(StatusIndicatorStyle::Kaomoji)
        );
        assert_eq!(
            StatusIndicatorStyle::parse("braille"),
            Some(StatusIndicatorStyle::Unicode)
        );
        assert!(StatusIndicatorStyle::parse("sparkle").is_none());
    }

    #[test]
    fn unicode_style_hides_verb() {
        let theme = Theme::default();
        let frame = render_indicator_frame(
            StatusIndicatorStyle::Unicode,
            &theme,
            TerminalGlyphProfile::Unicode,
            0,
            0,
        );
        assert!(!frame.show_verb);
        let emoji = render_indicator_frame(
            StatusIndicatorStyle::Emoji,
            &theme,
            TerminalGlyphProfile::Unicode,
            0,
            0,
        );
        assert!(emoji.show_verb);
    }

    #[test]
    fn emoji_frame_rotates() {
        let theme = Theme::default();
        let a = render_indicator_frame(
            StatusIndicatorStyle::Emoji,
            &theme,
            TerminalGlyphProfile::Unicode,
            0,
            0,
        );
        let b = render_indicator_frame(
            StatusIndicatorStyle::Emoji,
            &theme,
            TerminalGlyphProfile::Unicode,
            0,
            1,
        );
        assert_ne!(a.face, b.face);
    }
}
