//! Bot-challenge HTML detection — **diagnostics only** (not used in metasearch hot path).
//!
//! Python `ddgs` gates on HTTP status (`200` → parse, `403/429/…` → raise). It does not
//! inspect HTML for CAPTCHA strings. Metasearch follows that contract; these helpers remain
//! for tests, `/doctor`, and future observability.

/// True when the response looks like a DDG CAPTCHA interstitial (diagnostic).
pub fn is_bot_challenge(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("anomaly-modal")
        || lower.contains("bots use duckduckgo")
        || lower.contains("sorry, you have been blocked")
        || lower.contains("if this error persists") && lower.contains("duckduckgo.com")
}

/// True when HTML looks blocked **and** contains no parseable SERP markers (diagnostic).
pub fn is_engine_blocked(html: &str) -> bool {
    if html.contains("b_algo") || html.contains("result__a") {
        return false;
    }
    if is_bot_challenge(html) {
        return true;
    }
    let lower = html.to_ascii_lowercase();
    lower.contains("captchachallenge")
        || lower.contains("turnstile")
        || (lower.contains("captcha") && lower.contains("bing.com"))
        || (lower.contains("challenge") && lower.contains("verify you are human"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_anomaly_modal() {
        assert!(is_bot_challenge(
            r#"<div class="anomaly-modal__title">...</div>"#
        ));
    }

    #[test]
    fn real_results_page_is_not_challenge() {
        let html = r#"<a class="result__a" href="https://example.com">Example</a>"#;
        assert!(!is_bot_challenge(html));
    }

    #[test]
    fn detects_bing_captcha_page_without_serp() {
        assert!(is_engine_blocked(
            r#"<html><body>captcha challenge from bing.com</body></html>"#
        ));
    }

    #[test]
    fn bing_serp_with_turnstile_js_is_not_blocked() {
        let html = r#"
            <script>turnstile challenge widget</script>
            <li class="b_algo"><h2><a href="https://example.com">Example</a></h2><p>snippet</p></li>
        "#;
        assert!(!is_engine_blocked(html));
    }
}
