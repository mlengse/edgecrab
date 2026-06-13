//! Model selector catalog helpers — Hermes `modelPicker.tsx` data layer (DRY).

use std::collections::{BTreeMap, BTreeSet};

use edgecrab_core::{DiscoveryAvailability, DiscoverySource, ModelCatalog};

use crate::fuzzy_selector::{FuzzyItem, FuzzySelector};

/// A single model entry for fuzzy selector overlays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelEntry {
    pub display: String,
    pub provider: String,
    pub model_name: String,
    pub detail: String,
}

impl FuzzyItem for ModelEntry {
    fn primary(&self) -> &str {
        &self.display
    }

    fn secondary(&self) -> &str {
        &self.detail
    }

    fn tag(&self) -> &str {
        &self.provider
    }
}

pub fn discovery_source_label(source: DiscoverySource) -> &'static str {
    match source {
        DiscoverySource::Live => "live discovery",
        DiscoverySource::Cache => "cached discovery",
        DiscoverySource::Static => "static catalog",
    }
}

pub fn discovery_availability_short(availability: DiscoveryAvailability) -> String {
    match availability {
        DiscoveryAvailability::Supported => "live discovery".to_string(),
        DiscoveryAvailability::FeatureGated(feature) => {
            format!("live discovery gated by `{feature}`")
        }
        DiscoveryAvailability::Unsupported => "static catalog".to_string(),
    }
}

pub fn discovery_availability_detail(provider: &str, availability: DiscoveryAvailability) -> String {
    match availability {
        DiscoveryAvailability::Supported => {
            format!("{provider} supports live discovery in this build.")
        }
        DiscoveryAvailability::FeatureGated(feature) => format!(
            "{provider} supports live discovery, but this build falls back to the embedded catalog because `{feature}` is disabled."
        ),
        DiscoveryAvailability::Unsupported => {
            format!("{provider} is served from the embedded catalog.")
        }
    }
}

pub fn build_model_selector_entries(
    grouped: &[(String, Vec<String>)],
    dynamic_lookup: Option<&BTreeMap<String, (DiscoverySource, BTreeSet<String>)>>,
) -> Vec<ModelEntry> {
    let mut all_models = Vec::new();
    for (provider, models) in grouped {
        for model in models {
            let detail = match dynamic_lookup.and_then(|lookup| lookup.get(provider)) {
                Some((source, discovered_models)) if discovered_models.contains(model) => {
                    format!("{model} · {}", discovery_source_label(*source))
                }
                Some((DiscoverySource::Static, _)) => {
                    format!(
                        "{model} · {}",
                        discovery_source_label(DiscoverySource::Static)
                    )
                }
                Some(_) => format!("{model} · catalog fallback"),
                None => format!(
                    "{model} · {}",
                    discovery_source_label(DiscoverySource::Static)
                ),
            };
            all_models.push(ModelEntry {
                display: format!("{provider}/{model}"),
                provider: provider.clone(),
                detail,
                model_name: model.clone(),
            });
        }
    }
    all_models.sort_by(|left, right| left.display.cmp(&right.display));
    all_models
}

/// Footer hint for model selector chrome (Hermes type-to-filter discoverability).
pub fn model_selector_status_hint(
    selector: &FuzzySelector<ModelEntry>,
    refresh_in_flight: bool,
    current_model: &str,
) -> Option<String> {
    if refresh_in_flight {
        return Some("live discovery running".into());
    }
    let matched = selector.filtered.len();
    let total = selector.items.len();
    if matched == 0 && !selector.query.is_empty() {
        return Some("no matches — try provider or model fragment".into());
    }
    if matched < total && !selector.query.is_empty() {
        return Some(format!("{matched}/{total} matched"));
    }
    if !current_model.is_empty() {
        return Some(format!("current: {current_model}"));
    }
    None
}

pub fn build_models_inventory_report(
    providers: &[(String, Vec<String>)],
    current_model: &str,
    filter: &str,
) -> String {
    let current_provider = current_model
        .split_once('/')
        .map(|(provider, _)| edgecrab_core::normalize_discovery_provider(provider));
    let discovery_statuses: BTreeMap<String, DiscoveryAvailability> =
        edgecrab_core::discovery_provider_statuses()
            .into_iter()
            .collect();
    let mut text = if filter.is_empty() {
        "Model inventory (* = current provider):\n\n".to_string()
    } else {
        format!("Providers matching '{filter}' (* = current provider):\n\n")
    };

    for (provider, models) in providers {
        let label = ModelCatalog::provider_label(provider);
        let marker = if current_provider.as_deref() == Some(provider.as_str()) {
            " *"
        } else {
            ""
        };
        let availability = discovery_statuses
            .get(provider)
            .copied()
            .unwrap_or(DiscoveryAvailability::Unsupported);
        text.push_str(&format!(
            "  {provider:<12} {label:<22} {:>3} models  {}{marker}\n",
            models.len(),
            discovery_availability_short(availability),
        ));
    }

    text.push_str(
        "\nUse /models <provider> for the full list, /models refresh [provider|all] to refresh live inventories, or /model to open the selector.",
    );
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_sorted_entries() {
        let grouped = vec![
            ("openai".into(), vec!["gpt-4o".into()]),
            ("anthropic".into(), vec!["claude-opus-4.6".into()]),
        ];
        let entries = build_model_selector_entries(&grouped, None);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].display < entries[1].display);
    }

    #[test]
    fn fuzzy_tag_matches_provider() {
        let mut selector = FuzzySelector::new();
        selector.set_items(vec![ModelEntry {
            display: "anthropic/claude-opus-4.6".into(),
            provider: "anthropic".into(),
            model_name: "claude-opus-4.6".into(),
            detail: "static catalog".into(),
        }]);
        selector.query = "anthropic".into();
        selector.update_filter();
        assert_eq!(selector.filtered.len(), 1);
    }

    #[test]
    fn status_hint_on_filter_miss() {
        let mut selector = FuzzySelector::new();
        selector.set_items(vec![ModelEntry {
            display: "openai/gpt-4o".into(),
            provider: "openai".into(),
            model_name: "gpt-4o".into(),
            detail: "live".into(),
        }]);
        selector.query = "zzzzz".into();
        selector.update_filter();
        let hint = model_selector_status_hint(&selector, false, "openai/gpt-4o");
        assert!(hint.unwrap().contains("no matches"));
    }
}
