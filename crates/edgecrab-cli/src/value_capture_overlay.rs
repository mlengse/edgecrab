//! Pure value-capture overlay display helpers.

pub use crate::overlay_text_input::{
    map_overlay_text_input_key as map_value_capture_key,
    OverlayTextInputAction as ValueCaptureKeyAction,
};

/// Visible input line for the overlay (placeholder, masking, cursor handled separately).
pub fn value_capture_visible_text(buffer: &str, placeholder: &str, masked: bool) -> String {
    if buffer.is_empty() {
        placeholder.to_string()
    } else if masked {
        "•".repeat(buffer.chars().count())
    } else {
        buffer.to_string()
    }
}

pub fn value_capture_uses_placeholder(buffer: &str) -> bool {
    buffer.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_display_hides_chars() {
        assert_eq!(
            value_capture_visible_text("secret", "hint", true),
            "••••••"
        );
    }

    #[test]
    fn empty_shows_placeholder() {
        assert_eq!(
            value_capture_visible_text("", "type here", false),
            "type here"
        );
        assert!(value_capture_uses_placeholder(""));
    }
}
