//! Shared SERP text normalization — single place for HTML decode, tag strip, snippet hygiene.

use std::sync::OnceLock;

use regex::Regex;

const MAX_SNIPPET_CHARS: usize = 480;
const MAX_TITLE_CHARS: usize = 200;

/// Decode common HTML entities (including numeric `&#235;`).
pub fn decode_html_entities(input: &str) -> String {
    let mut out = input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");

    static DEC: OnceLock<Regex> = OnceLock::new();
    static HEX: OnceLock<Regex> = OnceLock::new();
    let dec = DEC.get_or_init(|| Regex::new(r"&#(\d+);").expect("dec entity"));
    let hex = HEX.get_or_init(|| Regex::new(r"&#x([0-9a-fA-F]+);").expect("hex entity"));

    out = dec
        .replace_all(&out, |caps: &regex::Captures| {
            caps.get(1)
                .and_then(|m| m.as_str().parse::<u32>().ok())
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .into_owned();
    out = hex
        .replace_all(&out, |caps: &regex::Captures| {
            caps.get(1)
                .and_then(|m| u32::from_str_radix(m.as_str(), 16).ok())
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .into_owned();
    out
}

/// Remove HTML tags and collapse whitespace.
pub fn strip_html_tags(html: &str) -> String {
    static TAG: OnceLock<Regex> = OnceLock::new();
    let re = TAG.get_or_init(|| Regex::new(r"<[^>]+>").expect("tag regex"));
    re.replace_all(html, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Title shown to the agent — decoded entities, no markup, bounded length.
pub fn clean_title(raw: &str) -> String {
    truncate_chars(
        strip_html_tags(&decode_html_entities(raw)).trim(),
        MAX_TITLE_CHARS,
    )
}

/// Snippet shown to the agent — drops Bing CSS/JS leakage, truncates long blobs.
pub fn clean_snippet(raw: &str) -> String {
    let decoded = decode_html_entities(raw);
    let stripped = strip_html_tags(&decoded);
    if is_markup_noise(&stripped) {
        return String::new();
    }
    truncate_chars(stripped.trim(), MAX_SNIPPET_CHARS)
}

/// True when scraped text is CSS/JS garbage rather than human-readable summary.
pub fn is_markup_noise(text: &str) -> bool {
    if text.len() < 40 {
        return false;
    }
    static NOISE: OnceLock<Regex> = OnceLock::new();
    let re = NOISE.get_or_init(|| {
        Regex::new(
            r"(?i)(\{[^}]*(?:color|flex-direction|padding|margin)\s*:|#b_results|\.b_imgcap|\.b_algo|var\(--|display\s*:\s*flex)",
        )
        .expect("noise regex")
    });
    re.is_match(text)
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let end = text
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= max)
        .last()
        .unwrap_or(0);
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_numeric_entity_in_title() {
        assert_eq!(clean_title("Rapha&#235;l MANSUY"), "Raphaël MANSUY");
    }

    #[test]
    fn snippet_drops_bing_css_leakage() {
        let css = ".b_imgcap_altitle{display:flex;flex-direction:row-reverse;gap:var(--smtc-padding)}";
        assert!(is_markup_noise(css));
        assert!(clean_snippet(css).is_empty());
    }

    #[test]
    fn keeps_normal_snippet() {
        let s = clean_snippet("Rust is a systems programming language focused on safety.");
        assert!(s.contains("systems programming"));
    }
}
