//! Query tokenization for opt-in ranked reorder (`DDGS_SELECTION=ranked` only).

use crate::tools::web::search::backend::SearchResult;

use super::text;

/// Whitespace-delimited tokens (NFKD-folded, length ≥ 3).
pub fn query_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(text::fold_for_search)
        .filter(|t| t.len() >= 3)
        .collect()
}

/// Count query tokens matched in title/snippet (word boundaries) or URL slug segments.
pub fn overlap_score(tokens: &[String], result: &SearchResult) -> u32 {
    if tokens.is_empty() {
        return 0;
    }
    let hay = text::fold_for_search(&format!("{} {}", result.title, result.snippet));
    tokens
        .iter()
        .filter(|t| token_in_text(&hay, t) || token_in_url(&result.url, t))
        .count() as u32
}

pub fn token_in_text(hay: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    hay.split(|c: char| !c.is_alphanumeric())
        .any(|word| word == token)
}

pub fn token_in_url(url: &str, token: &str) -> bool {
    if token.len() < 4 {
        return false;
    }
    text::fold_for_search(url)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .any(|seg| seg == token || seg.starts_with(token) || seg.ends_with(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_from_accented_name() {
        let t = query_tokens("Raphaël MANSUY");
        assert!(t.contains(&"raphael".to_string()));
        assert!(t.contains(&"mansuy".to_string()));
    }

    #[test]
    fn word_boundary_not_substring() {
        assert!(!token_in_text("smart contracts", "art"));
    }
}
