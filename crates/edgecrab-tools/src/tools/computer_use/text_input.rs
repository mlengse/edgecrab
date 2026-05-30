//! Text insertion strategy for computer_use.
//!
//! macOS `type_text` falls back to per-character CGEvent synthesis when AX bulk
//! insert fails (common in Safari/WebKit). That path cannot reliably produce
//! composed Unicode (e.g. `ë` → `eb`). Clipboard paste is the correct fix.

/// True when text should be inserted via clipboard paste rather than synthetic
/// per-character typing.
///
/// Per-character CGEvent typing is the failure-prone path: it cannot reliably
/// compose Unicode (`ë` → `eb`), and for long/multi-line content it floods
/// cua-driver's background daemon with hundreds of key events — observed to
/// trigger `daemon transport: daemon closed connection`. Clipboard paste is one
/// keystroke (`cmd+v`) and sidesteps both problems.
pub fn needs_clipboard_paste(text: &str) -> bool {
    !text.is_ascii()
        || text.contains('\n')
        || text.contains('\t')
        || text.chars().count() > 40
}

/// Heuristic: URL / domain typed into a browser omnibox after `cmd+l`.
pub fn looks_like_url_or_domain(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if t.starts_with("http://") || t.starts_with("https://") {
        return true;
    }
    t.contains('.')
        && !t.contains(' ')
        && t.chars().all(|c| c.is_ascii_alphanumeric() || ".-_:/?#%&=".contains(c))
}

/// True when a key combo focuses the browser address bar (Hermes workflow).
pub fn is_address_bar_focus_combo(keys: &str) -> bool {
    let lower = keys.to_ascii_lowercase();
    lower.contains("cmd+l") || lower.contains("command+l")
}

/// True for Return / Enter submit keys (normalized names).
pub fn is_submit_key(keys: &str) -> bool {
    matches!(
        keys.trim().to_ascii_lowercase().as_str(),
        "return" | "enter"
    )
}

/// Copy UTF-8 text to the macOS pasteboard via `pbcopy`.
#[cfg(target_os = "macos")]
pub fn copy_to_macos_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("pbcopy failed to start: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("pbcopy write failed: {e}"))?;
    }
    let status = child
        .wait()
        .map_err(|e| format!("pbcopy wait failed: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("pbcopy exited with {status}"))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn copy_to_macos_clipboard(_text: &str) -> Result<(), String> {
    Err("clipboard paste requires macOS".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_non_ascii_text() {
        assert!(!needs_clipboard_paste("Raphael MANSUY"));
        assert!(!needs_clipboard_paste("hello world 123"));
        assert!(needs_clipboard_paste("Raphaël MANSUY"));
        assert!(needs_clipboard_paste("café"));
        assert!(needs_clipboard_paste("日本語"));
    }

    #[test]
    fn pastes_long_or_multiline_ascii() {
        // Multi-line ASCII (e.g. a heredoc / script) must paste, not char-type.
        assert!(needs_clipboard_paste("python3 << 'EOF'\nimport subprocess\nEOF"));
        assert!(needs_clipboard_paste("col1\tcol2"));
        // Long single-line ASCII also pastes.
        assert!(needs_clipboard_paste(&"a".repeat(41)));
        // Short single-line ASCII still types normally.
        assert!(!needs_clipboard_paste(&"a".repeat(40)));
    }

    #[test]
    fn url_heuristic() {
        assert!(looks_like_url_or_domain("x.com"));
        assert!(looks_like_url_or_domain("https://x.com/home"));
        assert!(!looks_like_url_or_domain("hello world"));
    }

    #[test]
    fn address_bar_combo() {
        assert!(is_address_bar_focus_combo("cmd+l"));
        assert!(is_address_bar_focus_combo("Command+L"));
        assert!(!is_address_bar_focus_combo("Return"));
    }
}
