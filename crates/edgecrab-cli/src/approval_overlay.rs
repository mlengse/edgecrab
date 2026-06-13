//! Pure approval overlay key dispatch (Hermes `approvalAction` parity).

use crossterm::event::{KeyCode, KeyModifiers};
use edgecrab_core::ApprovalChoice;

pub const APPROVAL_LABELS: [&str; 4] = ["once", "session", "always", "deny"];
pub const APPROVAL_CHOICE_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalOverlayAction {
    SelectPrev,
    SelectNext,
    ScrollUp,
    ScrollDown,
    ToggleFullView,
    Confirm,
    Deny,
    Choose(usize),
    Noop,
}

pub fn approval_choice_at_index(index: usize) -> ApprovalChoice {
    match index {
        0 => ApprovalChoice::Once,
        1 => ApprovalChoice::Session,
        2 => ApprovalChoice::Always,
        _ => ApprovalChoice::Deny,
    }
}

/// Map a terminal key to an approval overlay action.
pub fn map_approval_key(code: KeyCode, modifiers: KeyModifiers) -> ApprovalOverlayAction {
    if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        return ApprovalOverlayAction::Noop;
    }

    match code {
        KeyCode::Left | KeyCode::Char('h') => ApprovalOverlayAction::SelectPrev,
        KeyCode::Right | KeyCode::Char('l') => ApprovalOverlayAction::SelectNext,
        KeyCode::Char('v') => ApprovalOverlayAction::ToggleFullView,
        KeyCode::Up | KeyCode::Char('k') => ApprovalOverlayAction::ScrollUp,
        KeyCode::Down | KeyCode::Char('j') => ApprovalOverlayAction::ScrollDown,
        KeyCode::Enter => ApprovalOverlayAction::Confirm,
        KeyCode::Esc => ApprovalOverlayAction::Deny,
        KeyCode::Char(c @ '1'..='4') => ApprovalOverlayAction::Choose((c as u8 - b'1') as usize),
        _ => ApprovalOverlayAction::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_maps_to_deny() {
        assert_eq!(
            map_approval_key(KeyCode::Esc, KeyModifiers::NONE),
            ApprovalOverlayAction::Deny
        );
    }

    #[test]
    fn number_keys_pick_choices() {
        assert_eq!(
            map_approval_key(KeyCode::Char('1'), KeyModifiers::NONE),
            ApprovalOverlayAction::Choose(0)
        );
        assert_eq!(
            map_approval_key(KeyCode::Char('4'), KeyModifiers::NONE),
            ApprovalOverlayAction::Choose(3)
        );
    }

    #[test]
    fn invalid_number_is_noop() {
        assert_eq!(
            map_approval_key(KeyCode::Char('0'), KeyModifiers::NONE),
            ApprovalOverlayAction::Noop
        );
        assert_eq!(
            map_approval_key(KeyCode::Char('5'), KeyModifiers::NONE),
            ApprovalOverlayAction::Noop
        );
    }

    #[test]
    fn enter_confirms() {
        assert_eq!(
            map_approval_key(KeyCode::Enter, KeyModifiers::NONE),
            ApprovalOverlayAction::Confirm
        );
    }

    #[test]
    fn horizontal_nav() {
        assert_eq!(
            map_approval_key(KeyCode::Left, KeyModifiers::NONE),
            ApprovalOverlayAction::SelectPrev
        );
        assert_eq!(
            map_approval_key(KeyCode::Right, KeyModifiers::NONE),
            ApprovalOverlayAction::SelectNext
        );
    }

    #[test]
    fn scroll_keys() {
        assert_eq!(
            map_approval_key(KeyCode::Up, KeyModifiers::NONE),
            ApprovalOverlayAction::ScrollUp
        );
        assert_eq!(
            map_approval_key(KeyCode::Down, KeyModifiers::NONE),
            ApprovalOverlayAction::ScrollDown
        );
    }

    #[test]
    fn choice_index_mapping() {
        assert_eq!(approval_choice_at_index(0), ApprovalChoice::Once);
        assert_eq!(approval_choice_at_index(3), ApprovalChoice::Deny);
    }
}
