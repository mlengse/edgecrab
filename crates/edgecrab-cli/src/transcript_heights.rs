//! Cached display-height estimates for transcript lines (Hermes `virtualHeights.ts` parity).

use std::collections::HashMap;

/// Default max chars per verbose tool trail block persisted in scrollback.
pub const VERBOSE_TRAIL_MAX_CHARS: usize = 800;
/// Default max lines per verbose tool trail block (Hermes `limits.ts`).
pub const VERBOSE_TRAIL_MAX_LINES: usize = 12;
/// Live streaming budget for in-flight tool args (Hermes `LIVE_RENDER_MAX_CHARS = 16_000`; shelf uses tighter cap).
pub const LIVE_RENDER_MAX_CHARS: usize = 512;

/// Hard cap on wrapped rows the height estimator counts (Hermes `MAX_ESTIMATE_LINES`).
pub const MAX_ESTIMATE_LINES: u16 = 800;

/// Estimate wrapped line count for monospace text at `width` columns.
pub fn estimate_wrapped_lines(text: &str, width: u16) -> u16 {
    estimate_wrapped_lines_capped(text, width, MAX_ESTIMATE_LINES)
}

/// Capped wrap estimate with byte/char budget bail for huge single-line messages.
pub fn estimate_wrapped_lines_capped(text: &str, width: u16, max_lines: u16) -> u16 {
    let w = usize::from(width.max(1));
    let max_lines = usize::from(max_lines.max(1));
    if text.is_empty() {
        return 1;
    }

    let char_budget = max_lines.saturating_mul(w).saturating_add(max_lines);
    let mut rows = 0usize;
    let mut line_chars = 0usize;
    let mut chars_seen = 0usize;

    for ch in text.chars() {
        chars_seen += 1;
        if chars_seen > char_budget && rows >= max_lines {
            return max_lines as u16;
        }
        if ch == '\n' {
            rows += 1;
            line_chars = 0;
        } else {
            line_chars += 1;
            if line_chars >= w {
                rows += 1;
                line_chars = 0;
            }
        }
        if rows >= max_lines {
            return max_lines as u16;
        }
    }
    if line_chars > 0 || rows == 0 {
        rows += 1;
    }
    rows.clamp(1, max_lines) as u16
}

/// Truncate verbose tool bodies for scrollback (Hermes `boundedLiveRenderText` persisted budget).
pub fn truncate_verbose_trail(text: &str) -> String {
    bounded_verbose_trail(text, VERBOSE_TRAIL_MAX_CHARS, VERBOSE_TRAIL_MAX_LINES)
}

/// Dual cap: last N lines and max chars, with omission prefix when truncated.
pub fn bounded_verbose_trail(text: &str, max_chars: usize, max_lines: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    let line_omitted = lines.len().saturating_sub(max_lines);
    let tail_lines: &[&str] = if line_omitted > 0 {
        &lines[lines.len() - max_lines..]
    } else {
        &lines[..]
    };

    let mut body = tail_lines.join("\n");
    let char_omitted = if body.chars().count() > max_chars {
        let start = body
            .char_indices()
            .nth(body.chars().count().saturating_sub(max_chars))
            .map(|(i, _)| i)
            .unwrap_or(0);
        body = format!("…{}", &body[start..]);
        true
    } else {
        false
    };

    match (line_omitted > 0, char_omitted) {
        (true, _) => format!("… ({line_omitted} lines omitted)\n{body}"),
        (false, true) => body,
        (false, false) => trimmed.to_string(),
    }
}

/// Tail-biased cap for streaming tool args (shows newest partial JSON).
pub fn bounded_live_render_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let start = trimmed
        .char_indices()
        .nth(trimmed.chars().count().saturating_sub(max_chars))
        .map(|(i, _)| i)
        .unwrap_or(0);
    format!("…{}", &trimmed[start..])
}

/// Per-line height cache keyed by stable line id (index or hash).
#[derive(Clone, Debug, Default)]
pub struct TranscriptHeightCache {
    heights: HashMap<u64, u16>,
}

impl TranscriptHeightCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_insert(&mut self, line_id: u64, width: u16, text: &str) -> u16 {
        if let Some(h) = self.heights.get(&line_id) {
            return *h;
        }
        let h = estimate_wrapped_lines(text, width);
        self.heights.insert(line_id, h);
        h
    }

    pub fn clear(&mut self) {
        self.heights.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_live_render_text_keeps_tail() {
        let text = format!("prefix-{{\"command\":\"{}\"}}", "x".repeat(600));
        let out = bounded_live_render_text(&text, 64);
        assert!(out.starts_with('…'));
        assert!(out.chars().count() <= 65);
        assert!(out.ends_with("\"}"));
    }

    #[test]
    fn giant_single_line_hits_cap_without_full_walk() {
        let giant = "x".repeat(2_000_000);
        let start = std::time::Instant::now();
        let rows = estimate_wrapped_lines(&giant, 80);
        assert_eq!(rows, MAX_ESTIMATE_LINES);
        assert!(
            start.elapsed().as_millis() < 500,
            "wrap estimate took too long"
        );
    }

    #[test]
    fn capped_wrap_respects_custom_max() {
        let text = "a".repeat(500);
        assert_eq!(estimate_wrapped_lines_capped(&text, 10, 5), 5);
    }

    #[test]
    fn wrap_counts_long_lines() {
        assert_eq!(estimate_wrapped_lines("hello", 80), 1);
        assert!(estimate_wrapped_lines(&"a".repeat(200), 40) >= 5);
    }

    #[test]
    fn cache_reuses_height() {
        let mut cache = TranscriptHeightCache::new();
        let h1 = cache.get_or_insert(1, 40, "short");
        let h2 = cache.get_or_insert(1, 40, "short");
        assert_eq!(h1, h2);
    }

    #[test]
    fn verbose_trail_truncates_chars() {
        let body = "z".repeat(2000);
        let out = truncate_verbose_trail(&body);
        assert!(out.starts_with('…'));
        assert!(out.chars().count() <= VERBOSE_TRAIL_MAX_CHARS + 1);
    }

    #[test]
    fn verbose_trail_truncates_lines() {
        let body: String = (1..=20).map(|n| format!("line{n}")).collect::<Vec<_>>().join("\n");
        let out = bounded_verbose_trail(&body, 10_000, 12);
        assert!(out.contains("lines omitted"));
        assert!(out.contains("line20"));
        assert!(!out.lines().any(|l| l == "line1"));
    }

    #[test]
    fn short_trail_unchanged() {
        let body = "alpha\nbeta\ngamma";
        assert_eq!(truncate_verbose_trail(body), body);
    }
}
