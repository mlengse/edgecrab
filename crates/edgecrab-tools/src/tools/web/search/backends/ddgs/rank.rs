//! Result ranking — merge engines, score by query fit, pick the best free-tier hits.

use std::collections::HashMap;

use crate::tools::web::search::backend::SearchResult;

use super::relevance;
use super::text;

const MIN_ACCEPT_SCORE: i32 = 1;
/// Stop probing extra engines once the pool has this many strong hits.
const SATISFIED_SCORE: i32 = 35;

/// Merge a batch into the pool (dedupe by URL, keep richer snippet).
pub fn extend_pool(pool: &mut Vec<SearchResult>, batch: Vec<SearchResult>) {
    let mut by_url: HashMap<String, SearchResult> = HashMap::new();
    for r in pool.drain(..) {
        let key = normalize_url_key(&r.url);
        by_url.insert(key, sanitize_result(r));
    }
    for r in batch {
        let key = normalize_url_key(&r.url);
        let cleaned = sanitize_result(r);
        by_url
            .entry(key)
            .and_modify(|existing| {
                if cleaned.snippet.len() > existing.snippet.len() {
                    existing.snippet = cleaned.snippet.clone();
                }
                if cleaned.title.len() > existing.title.len() {
                    existing.title = cleaned.title.clone();
                }
            })
            .or_insert(cleaned);
    }
    pool.extend(by_url.into_values());
}

/// Score, filter poison, sort, and return up to `max` hits (re-ranks 1..N).
pub fn rank_and_select(query: &str, pool: Vec<SearchResult>, max: usize) -> Vec<SearchResult> {
    if pool.is_empty() || max == 0 {
        return Vec::new();
    }

    let sanitized: Vec<_> = pool.into_iter().map(sanitize_result).collect();
    if relevance::is_likely_bot_spam(&sanitized) {
        return Vec::new();
    }

    let mut scored: Vec<(i32, SearchResult)> = sanitized
        .into_iter()
        .filter_map(|r| {
            let score = score_result(query, &r);
            (score >= MIN_ACCEPT_SCORE).then_some((score, r))
        })
        .collect();

    if scored.is_empty() {
        return Vec::new();
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.url.cmp(&b.1.url)));
    scored
        .into_iter()
        .take(max)
        .enumerate()
        .map(|(i, (_, r))| SearchResult::new(i + 1, r.title, r.url, r.snippet, r.source))
        .collect()
}

/// True when we already have enough high-confidence hits to skip slower engines.
pub fn pool_is_satisfied(query: &str, pool: &[SearchResult], max: usize) -> bool {
    if pool.len() < max {
        return false;
    }
    let mut scores: Vec<i32> = pool
        .iter()
        .map(|r| score_result(query, r))
        .filter(|&s| s >= MIN_ACCEPT_SCORE)
        .collect();
    scores.sort_by(|a, b| b.cmp(a));
    scores.len() >= max && scores[max - 1] >= SATISFIED_SCORE
}

fn sanitize_result(mut r: SearchResult) -> SearchResult {
    r.title = text::clean_title(&r.title);
    r.snippet = text::clean_snippet(&r.snippet);
    r
}

fn score_result(query: &str, result: &SearchResult) -> i32 {
    if relevance::is_spam_domain(&result.url) {
        return -1;
    }
    if text::is_markup_noise(&result.snippet) {
        return -1;
    }

    let mandatory = relevance::mandatory_tokens(query);
    let optional = relevance::optional_tokens(query);
    let hay = relevance::match_haystack(&result.title, &result.snippet, &result.url);

    if !mandatory.is_empty() && !mandatory.iter().all(|t| hay.contains(t)) {
        return -1;
    }

    let mut score = 10;

    for token in &mandatory {
        if hay.contains(token.as_str()) {
            score += 25;
        }
    }

    for token in &optional {
        if hay.contains(token.as_str()) {
            score += 12;
        }
        if relevance::fold_for_search(&result.title).contains(token.as_str()) {
            score += 8;
        }
    }

    if optional.is_empty() && mandatory.is_empty() {
        score += 5;
    }

    score += host_trust_bonus(&result.url);

    if result.snippet.is_empty() {
        score -= 5;
    } else if result.snippet.len() > 120 {
        score += 3;
    }

    score
}

fn host_trust_bonus(url: &str) -> i32 {
    let host = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .unwrap_or_default();

    if host.ends_with(".gov") || host.ends_with(".edu") {
        return 15;
    }
    if host.contains("wikipedia.org")
        || host.contains("github.com")
        || host.contains("linkedin.com")
        || host.contains("rust-lang.org")
        || host.contains("docs.rs")
    {
        return 12;
    }
    if host.ends_with(".org") {
        return 4;
    }
    0
}

fn normalize_url_key(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|mut u| {
            u.set_fragment(None);
            let host = u.host_str().unwrap_or("").to_ascii_lowercase();
            let path = u.path().trim_end_matches('/').to_ascii_lowercase();
            format!("{host}{path}")
        })
        .unwrap_or_else(|| url.trim().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::web::search::backend::SearchResult;

    fn hit(title: &str, snippet: &str, url: &str) -> SearchResult {
        SearchResult::new(1, title, url, snippet, "ddgs")
    }

    #[test]
    fn rank_prefers_linkedin_over_dexsport_for_person_query() {
        let query = "Raphaël MANSUY";
        let pool = vec![
            hit(
                "Dexsport crypto",
                "Web3 betting",
                "https://dexsport.io/",
            ),
            hit(
                "Rapha&#235;l MANSUY - LinkedIn",
                "Data engineering leader",
                "https://www.linkedin.com/in/raphaelmansuy",
            ),
        ];
        let out = rank_and_select(query, pool, 1);
        assert_eq!(out.len(), 1);
        assert!(out[0].url.contains("linkedin.com"));
    }

    #[test]
    fn merge_keeps_richer_snippet_for_same_url() {
        let mut pool = vec![hit(
            "Rust",
            "Short",
            "https://rust-lang.org/",
        )];
        extend_pool(
            &mut pool,
            vec![hit(
                "Rust",
                "Rust is a systems programming language empowering everyone.",
                "https://rust-lang.org/",
            )],
        );
        assert_eq!(pool.len(), 1);
        assert!(pool[0].snippet.contains("systems programming"));
    }

    #[test]
    fn satisfied_pool_skips_further_engine_probes() {
        let query = "Rust programming language";
        let pool = vec![
            hit(
                "Rust Programming Language",
                "Rust is fast and memory-efficient without a garbage collector.",
                "https://rust-lang.org/",
            ),
            hit(
                "Rust (programming language)",
                "General-purpose language emphasizing performance and safety.",
                "https://en.wikipedia.org/wiki/Rust_(programming_language)",
            ),
        ];
        assert!(pool_is_satisfied(query, &pool, 2));
    }
}
