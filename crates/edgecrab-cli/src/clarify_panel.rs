//! Clarify prompt formatting — Hermes `formatAbandonedClarify` parity.

/// Format a clarify prompt that was dismissed without an answer (timeout, /deny, interrupt).
pub fn format_abandoned_clarify(
    question: &str,
    choices: Option<&[String]>,
    reason: &str,
) -> String {
    let mut lines = vec![format!("❓ Clarify prompt ({reason}):"), format!("  Q: {question}")];
    if let Some(list) = choices {
        for (i, choice) in list.iter().enumerate() {
            lines.push(format!("  {}. {choice}", i + 1));
        }
        lines.push(format!("  {}. Other (type your answer)", list.len() + 1));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_choices_and_reason() {
        let out = format_abandoned_clarify(
            "Which API?",
            Some(&["REST".into(), "GraphQL".into()]),
            "timed out",
        );
        assert!(out.contains("timed out"));
        assert!(out.contains("Which API?"));
        assert!(out.contains("1. REST"));
        assert!(out.contains("Other"));
    }

    #[test]
    fn open_ended_omits_choice_list() {
        let out = format_abandoned_clarify("Describe the bug", None, "cancelled");
        assert!(!out.contains("1."));
        assert!(out.contains("Describe the bug"));
    }
}
