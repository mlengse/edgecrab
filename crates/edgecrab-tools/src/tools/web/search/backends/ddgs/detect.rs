//! Bot-challenge and unusable search HTML detection.

/// True when the response is a CAPTCHA / block page rather than search results (DDG).
pub fn is_bot_challenge(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("anomaly-modal")
        || lower.contains("bots use duckduckgo")
        || lower.contains("sorry, you have been blocked")
        || lower.contains("if this error persists") && lower.contains("duckduckgo.com")
        || (html.len() < 500
            && !html.contains("result__a")
            && !html.contains("b_algo")
            && lower.contains("duckduckgo"))
}

/// True when Bing (or generic engines) return a CAPTCHA interstitial instead of SERP HTML.
pub fn is_engine_blocked(html: &str) -> bool {
    // Ground truth: if organic result markers are present, the page is usable even when
    // embedded JS mentions turnstile/challenge (common on live Bing SERPs).
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
    fn detects_short_non_result_page() {
        assert!(is_bot_challenge(
            "<html><body>duckduckgo placeholder</body></html>"
        ));
    }

    #[test]
    fn real_results_page_is_not_challenge() {
        let html = r#"<a class="result__a" href="https://example.com">Example</a>"#;
        assert!(!is_bot_challenge(html));
    }

    #[test]
    fn detects_bing_captcha_page() {
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
