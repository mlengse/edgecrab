//! `/web` — opens the in-TUI chain editor (`web_setup_tui.rs`).
//! Shared chain/picker logic: `edgecrab_tools::WebChainEditor` in `tools/web/search/setup.rs`.
//! CLI wizard: `edgecrab setup web` → `web_setup.rs`.

/// Accent color for web overlays (warm amber).
pub const WEB_ACCENT: ratatui::style::Color = ratatui::style::Color::Rgb(255, 185, 75);
