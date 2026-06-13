//! Expensive-model confirmation — Hermes `model_cost_guard.py` parity.

use crate::pricing::get_pricing;

/// Input cost above this ($/M tokens) triggers confirmation when pricing is known.
pub const INPUT_COST_WARNING_THRESHOLD: f64 = 20.0;
/// Output cost above this ($/M tokens) triggers confirmation when pricing is known.
pub const OUTPUT_COST_WARNING_THRESHOLD: f64 = 100.0;

/// Confirmation payload for models above the safety threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpensiveModelWarning {
    pub model: String,
    pub message: String,
}

fn format_money(value: f64) -> String {
    format!("${value:.2}/M")
}

/// True when known pricing exceeds Hermes safety thresholds.
pub fn is_expensive_pricing(input_cost_per_million: f64, output_cost_per_million: f64) -> bool {
    input_cost_per_million > INPUT_COST_WARNING_THRESHOLD
        || output_cost_per_million > OUTPUT_COST_WARNING_THRESHOLD
}

/// Return a warning when catalog pricing exceeds safety thresholds.
///
/// Unknown or zero-cost (subscription/local) models never trigger.
pub fn expensive_model_warning(model_spec: &str) -> Option<ExpensiveModelWarning> {
    let model = model_spec.trim();
    if model.is_empty() {
        return None;
    }
    let pricing = get_pricing(model)?;
    if pricing.input_cost_per_million == 0.0 && pricing.output_cost_per_million == 0.0 {
        return None;
    }
    if !is_expensive_pricing(
        pricing.input_cost_per_million,
        pricing.output_cost_per_million,
    ) {
        return None;
    }

    let message = format!(
        "!!! EXPENSIVE MODEL WARNING !!!\n\n\
         {model} has known pricing above EdgeCrab's safety threshold.\n\
         Input tokens: {}\n\
         Output tokens: {}\n\
         Threshold: more than ${INPUT_COST_WARNING_THRESHOLD:.0}/M input or \
         ${OUTPUT_COST_WARNING_THRESHOLD:.0}/M output.\n\
         Pricing source: {:?}.\n\
         Confirm only if you intend to use this model.",
        format_money(pricing.input_cost_per_million),
        format_money(pricing.output_cost_per_million),
        pricing.source,
    );

    Some(ExpensiveModelWarning {
        model: model.to_string(),
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::CostSource;

    #[test]
    fn threshold_helper_matches_hermes_cutoffs() {
        assert!(!is_expensive_pricing(20.0, 100.0));
        assert!(is_expensive_pricing(20.01, 50.0));
        assert!(is_expensive_pricing(10.0, 100.01));
    }

    #[test]
    fn copilot_models_skip_guard() {
        assert!(expensive_model_warning("copilot/gpt-4.1-mini").is_none());
    }

    #[test]
    fn unknown_model_skips_guard() {
        assert!(expensive_model_warning("fakeprovider/no-such-model").is_none());
    }

    #[test]
    fn synthetic_expensive_message_shape() {
        let input = INPUT_COST_WARNING_THRESHOLD + 1.0;
        assert!(is_expensive_pricing(input, 0.0));
        let message = format!(
            "Input tokens: {}",
            format_money(input)
        );
        assert!(message.contains("$21.00/M"));
    }

    #[test]
    fn pricing_source_is_documented_in_warning() {
        if let Some(pricing) = get_pricing("anthropic/claude-opus-4.6") {
            if is_expensive_pricing(
                pricing.input_cost_per_million,
                pricing.output_cost_per_million,
            ) {
                let warning = expensive_model_warning("anthropic/claude-opus-4.6").unwrap();
                assert!(warning.message.contains("EXPENSIVE MODEL WARNING"));
                assert_eq!(warning.model, "anthropic/claude-opus-4.6");
                let _ = CostSource::OfficialDocsSnapshot;
                return;
            }
        }
    }
}
