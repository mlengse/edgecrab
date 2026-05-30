//! Result relevance — query tokenization and bot-spam batch detection.

use crate::tools::web::search::backend::SearchResult;

use super::text;

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "that", "this", "what", "who", "how", "about", "search",
    "sur", "les", "des", "une", "pour", "dans",
];

const SPAM_HOST_MARKERS: &[&str] = &[
    "dexsport",
    "tiktok.com",
    "dailymotion.com",
    "casino",
    "betting",
    "webcam",
];

/// Search query variants to try when the primary query yields irrelevant scrape results.
pub fn query_variants(query: &str) -> Vec<String> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }

    let mut out = vec![q.to_string()];
    let folded = fold_for_search(q);
    if !out.iter().any(|v| v.eq_ignore_ascii_case(&folded)) {
        out.push(folded.clone());
    }

    let titled = folded
        .split_whitespace()
        .map(title_case_word)
        .collect::<Vec<_>>()
        .join(" ");
    if !out.iter().any(|v| v.eq_ignore_ascii_case(&titled)) {
        out.push(titled);
    }

    out
}

/// Keep hits whose title or snippet plausibly match the user's query tokens.
pub fn filter_relevant(query: &str, results: Vec<SearchResult>) -> Vec<SearchResult> {
    if results.is_empty() || is_likely_bot_spam(&results) {
        return Vec::new();
    }

    let mandatory = mandatory_tokens(query);
    let optional = optional_tokens(query);

    if mandatory.is_empty() && optional.is_empty() {
        return results;
    }

    results
        .into_iter()
        .filter(|r| {
            !is_spam_domain(&r.url)
                && result_matches(&mandatory, &optional, r)
        })
        .collect()
}

fn result_matches(mandatory: &[String], optional: &[String], result: &SearchResult) -> bool {
    let hay = match_haystack(&result.title, &result.snippet, &result.url);

    if !mandatory.is_empty() && !mandatory.iter().all(|t| hay.contains(t)) {
        return false;
    }

    if optional.is_empty() {
        return true;
    }

    let matched = optional.iter().filter(|t| hay.contains(t.as_str())).count();
    if optional.len() == 1 {
        return matched >= 1;
    }
    matched >= optional.len().div_ceil(2).max(1)
}

/// True when Bing returns repeated placeholder titles (TikTok, Dailymotion, …).
pub fn is_likely_bot_spam(results: &[SearchResult]) -> bool {
    if results.len() < 3 {
        return false;
    }
    let first = fold_for_search(&results[0].title);
    let same_title = results
        .iter()
        .filter(|r| fold_for_search(&r.title) == first)
        .count();
    if same_title >= 3 {
        return true;
    }

    let domains: Vec<_> = results.iter().filter_map(|r| extract_host(&r.url)).collect();
    if domains.len() == results.len()
        && domains.len() >= 3
        && domains.windows(2).all(|w| w[0] == w[1])
    {
        return true;
    }

    false
}

pub fn is_spam_domain(url: &str) -> bool {
    let host = extract_host(url).unwrap_or_default();
    SPAM_HOST_MARKERS
        .iter()
        .any(|m| host.contains(m))
}

pub fn mandatory_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter(|w| is_emphasized_token(w))
        .map(|w| fold_for_search(w))
        .collect()
}

pub fn optional_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter_map(|w| {
            let folded = fold_for_search(w);
            if folded.len() < 3 || STOPWORDS.contains(&folded.as_str()) {
                None
            } else if is_emphasized_token(w) {
                None
            } else {
                Some(folded)
            }
        })
        .collect()
}

pub fn match_haystack(title: &str, snippet: &str, url: &str) -> String {
    fold_for_search(&format!("{title} {snippet} {url}"))
}

fn is_emphasized_token(word: &str) -> bool {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 3 {
        return false;
    }
    let upper = letters.iter().filter(|c| c.is_ascii_uppercase()).count();
    upper >= 2 && upper * 2 >= letters.len()
}

fn title_case_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

fn extract_host(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .map(|h| h.to_ascii_lowercase())
}

/// Accent-insensitive lowercase matching (European names).
pub fn fold_for_search(s: &str) -> String {
    text::decode_html_entities(s)
        .chars()
        .map(fold_char)
        .collect::<String>()
        .to_lowercase()
}

fn fold_char(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'a',
        'ç' | 'Ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
        'ñ' | 'Ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
        'ý' | 'ÿ' | 'Ý' => 'y',
        'æ' | 'Æ' => 'a',
        'œ' | 'Œ' => 'o',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::web::search::backend::SearchResult;

    fn hit(title: &str, snippet: &str, url: &str) -> SearchResult {
        SearchResult::new(1, title, url, snippet, "ddgs")
    }

    #[test]
    fn filters_bot_spam_dexsport_for_person_query() {
        let query = "Raphaël MANSUY";
        let poisoned = vec![
            hit("Dexsport crypto betting", "Web3 sportsbook", "https://dexsport.io/"),
            hit("Dexsport Review", "No KYC", "https://example.com/dex"),
            hit("Dexsport Price", "DESU token", "https://cryptoslate.com/dex"),
        ];
        assert!(is_likely_bot_spam(&poisoned) || filter_relevant(query, poisoned).is_empty());
    }

    #[test]
    fn rejects_painter_raphael_when_surname_mansuy_required() {
        let query = "Raphaël MANSUY";
        let wrong_person = vec![hit(
            "Raphaël (peintre) — Wikipédia",
            "Peintre italien",
            "https://fr.wikipedia.org/wiki/Rapha%C3%ABl_(peintre)",
        )];
        assert!(filter_relevant(query, wrong_person).is_empty());
    }

    #[test]
    fn keeps_rust_programming_results() {
        let query = "Rust programming language";
        let good = vec![hit(
            "Rust Programming Language",
            "Rust is a systems programming language",
            "https://rust-lang.org/",
        )];
        let kept = filter_relevant(query, good);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn detects_identical_title_spam_batch() {
        let spam = (0..4)
            .map(|i| hit("TikTok - Make Your Day", "watch videos", &format!("https://tiktok.com/{i}")))
            .collect::<Vec<_>>();
        assert!(is_likely_bot_spam(&spam));
    }

    #[test]
    fn query_variants_includes_accent_folded_form() {
        let v = query_variants("Raphaël MANSUY");
        assert!(v.iter().any(|q| q.contains("raphael") || q.contains("Raphael")));
        assert!(v[0].contains('ë') || v[0].contains("MANSUY"));
    }

    #[test]
    fn mandatory_token_extraction_from_all_caps_surname() {
        let m = mandatory_tokens("Raphaël MANSUY");
        assert!(m.iter().any(|t| t == "mansuy"));
        assert!(!m.iter().any(|t| t == "raphael"));
    }

    #[test]
    fn matches_bing_html_entity_title_and_linkedin_url() {
        let query = "Raphaël MANSUY";
        let hits = vec![hit(
            "Rapha&#235;l MANSUY - LinkedIn",
            "Data engineering profile",
            "https://hk.linkedin.com/in/raphaelmansuy",
        )];
        assert_eq!(filter_relevant(query, hits).len(), 1);
    }

    #[test]
    fn emphasized_token_requires_majority_uppercase() {
        assert!(is_emphasized_token("MANSUY"));
        assert!(is_emphasized_token("NASA"));
        assert!(!is_emphasized_token("Raphaël"));
        assert!(!is_emphasized_token("Rust"));
    }

    #[test]
    fn spam_domain_blocks_dexsport_host() {
        assert!(is_spam_domain("https://dexsport.io/page"));
        assert!(!is_spam_domain("https://rust-lang.org/"));
    }
}
