//! Result selection — Python `ddgs` raw contract + optional overlap reorder (no row drops).

use std::collections::HashMap;

use crate::tools::web::search::backend::SearchResult;

use super::parse;
use super::query;
use super::settings::DdgsSelectionMode;
use super::text;

/// Merge batches — dedupe by URL (Python `cache` set), preserve first-seen SERP order.
pub fn extend_pool(pool: &mut Vec<SearchResult>, batch: Vec<SearchResult>) {
    let mut index_by_url: HashMap<String, usize> = HashMap::new();

    for (i, r) in pool.iter().enumerate() {
        index_by_url.insert(normalize_url_key(&r.url), i);
    }

    for r in pool.iter_mut() {
        *r = sanitize_result(r.clone());
    }

    for r in batch {
        let cleaned = sanitize_result(r);
        let key = normalize_url_key(&cleaned.url);
        if let Some(&idx) = index_by_url.get(&key) {
            merge_richer(&mut pool[idx], cleaned);
        } else {
            index_by_url.insert(key, pool.len());
            pool.push(cleaned);
        }
    }
}

/// Apply selection policy after metasearch (mode from [`DdgsSelectionMode`]).
pub fn select_results(
    mode: DdgsSelectionMode,
    query: &str,
    pool: Vec<SearchResult>,
    max: usize,
) -> Vec<SearchResult> {
    match mode {
        DdgsSelectionMode::Raw => select_raw(pool, max),
        DdgsSelectionMode::Ranked => select_ranked(query, pool, max),
    }
}

/// Python `DDGS.text()` contract: keep SERP order, cap at `max` (filtering done per-engine at parse).
pub fn select_raw(pool: Vec<SearchResult>, max: usize) -> Vec<SearchResult> {
    deliverable_indexed(pool, false)
        .into_iter()
        .take(max)
        .enumerate()
        .map(|(i, (_, r))| SearchResult::new(i + 1, r.title, r.url, r.snippet, r.source))
        .collect()
}

/// Opt-in EdgeCrab mode: same deliverability filter, reorder by query-token overlap — never drop rows.
pub fn select_ranked(query: &str, pool: Vec<SearchResult>, max: usize) -> Vec<SearchResult> {
    let tokens = query::query_tokens(query);
    let mut indexed = deliverable_indexed(pool, true);

    if !tokens.is_empty() {
        indexed.sort_by(|(ia, a), (ib, b)| {
            query::overlap_score(&tokens, b)
                .cmp(&query::overlap_score(&tokens, a))
                .then_with(|| ia.cmp(ib))
        });
    }

    indexed
        .into_iter()
        .take(max)
        .enumerate()
        .map(|(i, (_, r))| SearchResult::new(i + 1, r.title, r.url, r.snippet, r.source))
        .collect()
}

fn deliverable_indexed(pool: Vec<SearchResult>, ranked: bool) -> Vec<(usize, SearchResult)> {
    pool.into_iter()
        .enumerate()
        .map(|(i, r)| (i, sanitize_result(r)))
        .filter(|(_, r)| is_deliverable_row(r, ranked))
        .collect()
}

fn merge_richer(existing: &mut SearchResult, incoming: SearchResult) {
    if incoming.snippet.len() > existing.snippet.len() {
        existing.snippet = incoming.snippet;
    }
    if incoming.title.len() > existing.title.len() {
        existing.title = incoming.title;
    }
}

/// Python raw mode: non-empty URL only. Ranked mode also drops HTML/Lite ad rows if they leaked in.
pub fn is_deliverable(r: &SearchResult) -> bool {
    is_deliverable_row(r, false)
}

fn is_deliverable_row(r: &SearchResult, ranked: bool) -> bool {
    if r.url.trim().is_empty() {
        return false;
    }
    if ranked && parse::is_non_result_url(&r.url) {
        return false;
    }
    true
}

fn sanitize_result(mut r: SearchResult) -> SearchResult {
    r.title = text::normalize_field(&r.title);
    r.snippet = text::normalize_field(&r.snippet);
    r.url = text::normalize_url_field(&r.url);
    r
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
    use crate::tools::web::search::backends::ddgs::settings::DdgsSelectionMode;

    fn hit(title: &str, snippet: &str, url: &str) -> SearchResult {
        SearchResult::new(1, title, url, snippet, "ddgs")
    }

    #[test]
    fn raw_matches_python_serp_order() {
        let pool = vec![
            hit("Ad", "buy", "https://duckduckgo.com/y.js?ad_domain=x"),
            hit("Rust", "lang", "https://rust-lang.org/"),
            hit("Book", "learn", "https://doc.rust-lang.org/book/"),
        ];
        let out = select_raw(pool, 5);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].url, "https://duckduckgo.com/y.js?ad_domain=x");
        assert_eq!(out[1].url, "https://rust-lang.org/");
    }

    #[test]
    fn raw_returns_poison_serp_like_python() {
        let pool = vec![
            hit("Dexsport", "a", "https://dexsport.io/"),
            hit("Dexsport 2", "b", "https://example.com/dex"),
        ];
        assert_eq!(select_raw(pool, 5).len(), 2);
    }

    #[test]
    fn ranked_reorders_without_dropping_unrelated() {
        let pool = vec![
            hit("Dexsport", "a", "https://dexsport.io/"),
            hit(
                "Raphaël MANSUY",
                "profile",
                "https://linkedin.com/in/raphaelmansuy",
            ),
        ];
        let out = select_ranked("Raphaël MANSUY", pool, 2);
        assert_eq!(out.len(), 2);
        assert!(out[0].url.contains("linkedin.com"));
    }

    #[test]
    fn merge_preserves_serp_order() {
        let mut pool = vec![hit("A", "a", "https://a.example/")];
        extend_pool(
            &mut pool,
            vec![
                hit("B", "b", "https://b.example/"),
                hit("A2", "aa", "https://a.example/"),
            ],
        );
        assert_eq!(pool[0].url, "https://a.example/");
        assert_eq!(pool[1].url, "https://b.example/");
    }

    #[test]
    fn select_results_dispatches_mode() {
        let pool = vec![hit("X", "s", "https://x.example/")];
        assert_eq!(
            select_results(DdgsSelectionMode::Raw, "", pool.clone(), 1).len(),
            1
        );
        assert_eq!(
            select_results(DdgsSelectionMode::Ranked, "x", pool, 1).len(),
            1
        );
    }
}
