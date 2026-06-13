//! Pure secret/sudo capture overlay helpers (Hermes `MaskedPrompt` parity).

pub use crate::overlay_text_input::map_overlay_text_input_key;

/// Mask typed secret by character count — never echo plaintext in the TUI.
pub fn secret_masked_display(char_count: usize) -> String {
    "•".repeat(char_count)
}

pub fn secret_prompt_icon(is_sudo: bool) -> &'static str {
    if is_sudo { "🔒" } else { "🔑" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_display_hides_length() {
        assert_eq!(secret_masked_display(8), "••••••••");
        assert!(secret_masked_display(0).is_empty());
    }

    #[test]
    fn sudo_uses_lock_icon() {
        assert_eq!(secret_prompt_icon(true), "🔒");
        assert_eq!(secret_prompt_icon(false), "🔑");
    }
}
