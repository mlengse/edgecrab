//! Browser-specific navigation helpers (cua-driver WEB_APPS.md parity).

use super::text_input::looks_like_url_or_domain;

/// Normalize a domain or partial URL for `launch_app({ urls: [...] })`.
pub fn normalize_nav_url(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() {
        return String::new();
    }
    if t.starts_with("http://") || t.starts_with("https://") {
        return t.to_string();
    }
    format!("https://{t}")
}

/// True when the app name looks like a Chromium/WebKit browser.
pub fn is_browser_app(app: &str) -> bool {
    let lower = app.to_ascii_lowercase();
    [
        "chrome",
        "google chrome",
        "safari",
        "firefox",
        "arc",
        "brave",
        "edge",
        "microsoft edge",
        "vivaldi",
        "opera",
        "dia",
        "chromium",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// Bundle ID for `launch_app` when opening a URL in the background.
pub fn browser_bundle_id(app: &str) -> Option<&'static str> {
    let lower = app.to_ascii_lowercase();
    if lower.contains("chrome") || lower.contains("chromium") {
        return Some("com.google.Chrome");
    }
    if lower.contains("safari") {
        return Some("com.apple.Safari");
    }
    if lower.contains("firefox") {
        return Some("org.mozilla.firefox");
    }
    if lower.contains("arc") {
        return Some("company.thebrowser.Browser");
    }
    if lower.contains("brave") {
        return Some("com.brave.Browser");
    }
    if lower.contains("edge") {
        return Some("com.microsoft.edgemac");
    }
    if lower.contains("vivaldi") {
        return Some("com.vivaldi.Vivaldi");
    }
    if lower.contains("opera") {
        return Some("com.operasoftware.Opera");
    }
    None
}

/// Resolve launch target: explicit `bundle_id` wins, then `app` name, then `last_app`.
pub fn resolve_launch_target(
    app: Option<&str>,
    bundle_id: Option<&str>,
    last_app: Option<&str>,
) -> String {
    if let Some(b) = bundle_id.filter(|s| s.contains('.')) {
        return b.to_string();
    }
    if let Some(app) = app {
        if let Some(b) = browser_bundle_id(app) {
            return b.to_string();
        }
        if app.contains('.') {
            return app.to_string();
        }
    }
    if let Some(last) = last_app {
        if let Some(b) = browser_bundle_id(last) {
            return b.to_string();
        }
    }
    "com.google.Chrome".to_string()
}

/// Whether this action should open a URL via `launch_app` (not omnibox typing).
pub fn should_open_url_via_launch(app: Option<&str>, last_app: Option<&str>, text: &str) -> bool {
    if !looks_like_url_or_domain(text) {
        return false;
    }
    app.or(last_app)
        .map(is_browser_app)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_adds_https() {
        assert_eq!(normalize_nav_url("x.com"), "https://x.com");
        assert_eq!(
            normalize_nav_url("https://x.com/home"),
            "https://x.com/home"
        );
    }

    #[test]
    fn detects_chrome() {
        assert!(is_browser_app("Google Chrome"));
        assert_eq!(
            browser_bundle_id("Google Chrome"),
            Some("com.google.Chrome")
        );
    }

    #[test]
    fn should_launch_when_app_and_url() {
        assert!(should_open_url_via_launch(
            Some("Google Chrome"),
            None,
            "x.com"
        ));
        assert!(!should_open_url_via_launch(Some("Notes"), None, "x.com"));
    }

    #[test]
    fn resolve_target_prefers_bundle_id() {
        assert_eq!(
            resolve_launch_target(
                Some("Google Chrome"),
                Some("com.google.Chrome"),
                None
            ),
            "com.google.Chrome"
        );
    }
}
