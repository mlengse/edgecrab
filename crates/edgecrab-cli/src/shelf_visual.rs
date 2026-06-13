//! Shelf visual helpers — Hermes `subagentTree.ts` parity (elapsed heat, sparklines).
//!
//! Pure functions only; no ratatui dependency (easy to unit test).

use ratatui::style::Color;

/// Elapsed-time severity bucket for shelf rows (aligns with long-run charm @ 8s).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElapsedHeat {
    Calm,
    Warm,
    Hot,
}

const SPARK_RAMP: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Format seconds as `Ns`, `Nm`, or `Nm Ss` (Hermes `fmtDuration`).
pub fn fmt_duration(secs: u64) -> String {
    if secs < 60 {
        return format!("{secs}s");
    }
    let m = secs / 60;
    let s = secs % 60;
    if s == 0 {
        format!("{m}m")
    } else {
        format!("{m}m {s}s")
    }
}

/// Map elapsed seconds to a heat bucket (8s warm, 30s hot — matches long-run hints).
pub fn elapsed_heat(secs: u64) -> ElapsedHeat {
    if secs >= 30 {
        ElapsedHeat::Hot
    } else if secs >= 8 {
        ElapsedHeat::Warm
    } else {
        ElapsedHeat::Calm
    }
}

/// Pick a color for elapsed suffix from theme palette.
pub fn heat_color(heat: ElapsedHeat, dim: Color, warn: Color, hot: Color) -> Color {
    match heat {
        ElapsedHeat::Calm => dim,
        ElapsedHeat::Warm => warn,
        ElapsedHeat::Hot => hot,
    }
}

/// 8-step unicode bar sparkline from positive integers (Hermes `sparkline`).
pub fn sparkline(values: &[u64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let max = values.iter().copied().max().unwrap_or(0);
    if max == 0 {
        return " ".repeat(values.len());
    }
    values
        .iter()
        .map(|&v| {
            if v == 0 {
                return " ".to_string();
            }
            let idx = ((v as f64 / max as f64) * (SPARK_RAMP.len() - 1) as f64).ceil() as usize;
            let idx = idx.min(SPARK_RAMP.len() - 1);
            SPARK_RAMP[idx].to_string()
        })
        .collect()
}

/// Join recent tool names for sub-agent output tail (`file_read · terminal · grep`).
pub fn format_recent_tools(tools: &[String], max: usize) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let start = tools.len().saturating_sub(max);
    tools[start..].join(" · ")
}

/// Rough token estimate (~4 chars/token) — Hermes `estimateTokensRough`.
pub fn estimate_tokens_rough(text: &str) -> u32 {
    let len = text.len();
    if len == 0 {
        return 0;
    }
    len.div_ceil(4) as u32
}

/// Compact count for shelf labels — Hermes `fmtK` (`12k`, `1.5k`, `999`).
pub fn fmt_k(n: u64) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    if n < 10_000 {
        let whole = n / 1_000;
        let frac = (n % 1_000) / 100;
        if frac == 0 {
            return format!("{whole}k");
        }
        return format!("{whole}.{frac}k");
    }
    if n < 1_000_000 {
        let whole = n / 1_000;
        return format!("{whole}k");
    }
    let whole = n / 1_000_000;
    let frac = (n % 1_000_000) / 100_000;
    if frac == 0 {
        format!("{whole}m")
    } else {
        format!("{whole}.{frac}m")
    }
}

/// Shelf suffix like `~12k tokens` (Hermes thinking/tools headers).
pub fn format_tokens_label(count: u32) -> Option<String> {
    if count == 0 {
        return None;
    }
    Some(format!("~{} tokens", fmt_k(count as u64)))
}

/// Footer when both thinking and tool estimates are present — `Σ ~15k total`.
pub fn format_tokens_total(thinking: u32, tools: u32) -> Option<String> {
    if thinking > 0 && tools > 0 {
        Some(format!(
            "Σ ~{} total",
            fmt_k(thinking as u64 + tools as u64)
        ))
    } else {
        None
    }
}

/// Section chevron — Hermes `▸` collapsed / `▾` expanded.
pub fn section_chevron(expanded: bool) -> &'static str {
    if expanded {
        "▾ "
    } else {
        "▸ "
    }
}

/// Status glyph for completed delegates in `/agents` history.
pub fn delegate_status_glyph(status: &str) -> char {
    match status.to_ascii_lowercase().as_str() {
        "completed" | "done" | "success" => '✓',
        "error" | "failed" => '✗',
        "interrupted" | "timeout" | "cancelled" => '⏸',
        "running" => '◐',
        _ => '•',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_duration_seconds_and_minutes() {
        assert_eq!(fmt_duration(4), "4s");
        assert_eq!(fmt_duration(90), "1m 30s");
        assert_eq!(fmt_duration(120), "2m");
    }

    #[test]
    fn elapsed_heat_thresholds() {
        assert_eq!(elapsed_heat(7), ElapsedHeat::Calm);
        assert_eq!(elapsed_heat(8), ElapsedHeat::Warm);
        assert_eq!(elapsed_heat(29), ElapsedHeat::Warm);
        assert_eq!(elapsed_heat(30), ElapsedHeat::Hot);
    }

    #[test]
    fn sparkline_ramps() {
        assert_eq!(sparkline(&[]), "");
        assert_eq!(sparkline(&[0, 0]), "  ");
        let out = sparkline(&[1, 8]);
        assert_eq!(out.chars().count(), 2);
        assert!(out.contains('█') || out.contains('▇'));
    }

    #[test]
    fn format_recent_tools_takes_tail() {
        let tools = vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "e".into(),
        ];
        assert_eq!(format_recent_tools(&tools, 3), "c · d · e");
    }

    #[test]
    fn estimate_tokens_rough_ceil() {
        assert_eq!(estimate_tokens_rough(""), 0);
        assert_eq!(estimate_tokens_rough("abcd"), 1);
        assert_eq!(estimate_tokens_rough("abcdefgh"), 2);
    }

    #[test]
    fn fmt_k_compact() {
        assert_eq!(fmt_k(999), "999");
        assert_eq!(fmt_k(1_000), "1k");
        assert_eq!(fmt_k(1_500), "1.5k");
        assert_eq!(fmt_k(12_000), "12k");
    }

    #[test]
    fn format_tokens_total_requires_both() {
        assert!(format_tokens_total(100, 0).is_none());
        assert_eq!(
            format_tokens_total(1_000, 2_000).unwrap(),
            "Σ ~3k total"
        );
    }
}
