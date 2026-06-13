//! Shared picker list chrome (chevron marker, padding).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

/// List-row selection chevron used across fuzzy pickers and mode selectors.
pub fn selector_marker(is_selected: bool, accent: Color, bg: Option<Color>) -> Span<'static> {
    let mut style = Style::default().fg(if is_selected { accent } else { Color::DarkGray });
    if let Some(bg) = bg {
        style = style.bg(bg);
    }
    if is_selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(if is_selected { "▶ " } else { "  " }, style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_marker_uses_chevron() {
        let span = selector_marker(true, Color::Cyan, None);
        assert!(span.content.contains('▶'));
    }
}
