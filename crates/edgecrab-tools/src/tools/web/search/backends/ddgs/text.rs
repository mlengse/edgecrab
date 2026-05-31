//! Shared SERP text normalization — Python `ddgs.utils._normalize` / `_normalize_url` parity.

use std::sync::OnceLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

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

/// Remove HTML tags — Python `REGEX_STRIP_TAGS.sub("", raw_html)`.
pub fn strip_html_tags(html: &str) -> String {
    static TAG: OnceLock<Regex> = OnceLock::new();
    let re = TAG.get_or_init(|| Regex::new(r"<[^>]+>").expect("tag regex"));
    re.replace_all(html, "").into_owned()
}

/// Python `_normalize`: unescape + strip tags (no truncation, no CSS heuristics).
pub fn normalize_field(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let decoded = decode_html_entities(raw);
    strip_html_tags(&decoded).replace('\u{00a0}', " ")
}

/// Python `_normalize_url`: unquote + replace spaces with `+`.
pub fn normalize_url_field(url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    percent_unquote(url).replace(' ', "+")
}

fn percent_unquote(input: &str) -> String {
    let mut bytes = Vec::new();
    let b = input.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(hex) = std::str::from_utf8(&b[i + 1..i + 3])
                && let Ok(byte) = u8::from_str_radix(hex, 16)
            {
                bytes.push(byte);
                i += 3;
                continue;
            }
        } else if b[i] == b'+' {
            bytes.push(b' ');
            i += 1;
            continue;
        }
        bytes.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Back-compat aliases used by parsers.
pub fn clean_title(raw: &str) -> String {
    normalize_field(raw).trim().to_string()
}

pub fn clean_snippet(raw: &str) -> String {
    normalize_field(raw).trim().to_string()
}

/// Accent-insensitive case fold (Unicode NFKD) — ranked mode only.
pub fn fold_for_search(s: &str) -> String {
    decode_html_entities(s)
        .nfkd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_strips_accents() {
        assert_eq!(fold_for_search("Raphaël"), "raphael");
        assert_eq!(fold_for_search("café"), "cafe");
    }

    #[test]
    fn decodes_numeric_entity_in_title() {
        assert_eq!(normalize_field("Rapha&#235;l MANSUY"), "Raphaël MANSUY");
    }

    #[test]
    fn normalize_field_strips_tags_like_python() {
        assert_eq!(
            normalize_field("<strong>Rust</strong> lang"),
            "Rust lang"
        );
    }

    #[test]
    fn normalize_url_replaces_spaces() {
        assert_eq!(
            normalize_url_field("https://example.com/a%20b"),
            "https://example.com/a+b"
        );
    }
}
