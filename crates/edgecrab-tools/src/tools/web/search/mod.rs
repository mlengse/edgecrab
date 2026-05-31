//! Pluggable web search backends with fallback chain.

pub mod backend;
pub mod backend_settings;
pub mod backends;
pub mod chain;
pub mod config;
pub mod content_crawl;
pub mod content_extract;
pub mod error;
pub mod http;
pub mod provider_capabilities;
pub mod provider_diagnostics;
pub mod rate_limit;
pub mod registry;
pub mod response;
pub mod setup_schema;
pub mod setup;
pub mod tool;
pub mod tui;
pub mod web_config;

#[cfg(test)]
mod test_isolation;

pub use backend::{SearchResult, WebSearchBackend};
pub use config::{
    ExtractOptions, ResolvedChain, SearchOptions, WebSearchChainUpdate,
    clear_web_search_chain_in_config, format_search_chain_summary,
    load_web_search_config_from_disk, load_web_search_config_from_path,
    persist_web_search_chain_in_config, web_search_is_available,
};
pub use error::SearchError;
pub use provider_diagnostics::{
    WebDiagnosticsReport, WebProviderStatus, capability_label, collect_web_diagnostics,
    format_extract_doctor_detail, format_search_doctor_detail, format_web_setup_report,
    web_provider_picker_rows,
};
pub use registry::{
    extract_with_backend, get_web_search_backend, list_web_provider_setup_schemas,
    list_web_search_backends, register_web_search_backend,
};
pub use setup::{
    ChainEditError, WebChainEditor, WebPickerCatalog, active_search_section_override,
    chain_eligible_search_ids, chain_summary_after_save, default_chain_for_backend,
    format_picker_label, ensure_web_search_config_coherence,
    ensure_web_search_config_coherence_at, migrate_legacy_search_override,
    order_from_primary_and_fallbacks, persist_search_backend_as_chain, persist_search_chain_order, persist_search_chain_with_timeout,
    print_provider_detail_cli, primary_and_fallbacks_from_order, provider_detail_lines,
    reset_web_to_auto, search_override_warning, search_section_override_from_path,
};
pub use tui::{
    gateway_web_command_reply, render_web_dashboard, web_command_overlay, web_command_usage,
    web_menu_status_hint, web_status_one_liner,
};
pub use tool::WebSearchTool;
pub use web_config::{
    WebSectionUpdate, clear_web_search_section_overrides, clear_web_section_overrides,
    load_web_tools_config_from_disk, load_web_tools_config_from_path, persist_web_backend_in_config,
    persist_web_section_in_config, resolve_config_extract_backend, resolve_config_search_backend,
};

#[cfg(test)]
mod tests;
