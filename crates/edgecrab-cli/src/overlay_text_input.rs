//! Shared single-line overlay text input key dispatch (approval uses separate matrix).

use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayTextInputAction {
    AppendChar(char),
    Backspace,
    Submit,
    Cancel,
    Noop,
}

pub fn map_overlay_text_input_key(
    code: KeyCode,
    modifiers: KeyModifiers,
) -> OverlayTextInputAction {
    if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        return OverlayTextInputAction::Noop;
    }

    match code {
        KeyCode::Char(c) => OverlayTextInputAction::AppendChar(c),
        KeyCode::Backspace => OverlayTextInputAction::Backspace,
        KeyCode::Enter => OverlayTextInputAction::Submit,
        KeyCode::Esc => OverlayTextInputAction::Cancel,
        _ => OverlayTextInputAction::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printable_char_appends() {
        assert_eq!(
            map_overlay_text_input_key(KeyCode::Char('a'), KeyModifiers::NONE),
            OverlayTextInputAction::AppendChar('a')
        );
    }

    #[test]
    fn ctrl_char_is_noop() {
        assert_eq!(
            map_overlay_text_input_key(KeyCode::Char('a'), KeyModifiers::CONTROL),
            OverlayTextInputAction::Noop
        );
    }

    #[test]
    fn enter_submits_esc_cancels() {
        assert_eq!(
            map_overlay_text_input_key(KeyCode::Enter, KeyModifiers::NONE),
            OverlayTextInputAction::Submit
        );
        assert_eq!(
            map_overlay_text_input_key(KeyCode::Esc, KeyModifiers::NONE),
            OverlayTextInputAction::Cancel
        );
    }

    #[test]
    fn backspace_edits() {
        assert_eq!(
            map_overlay_text_input_key(KeyCode::Backspace, KeyModifiers::NONE),
            OverlayTextInputAction::Backspace
        );
    }
}
