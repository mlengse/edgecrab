//! Web tools — pluggable search, extract, and crawl.

mod extract_crawl;
pub mod search;

pub use extract_crawl::{WebCrawlTool, WebExtractTool};
pub use search::{
    SearchResult, WebSearchBackend, WebSearchChainUpdate, WebSearchTool, WebSectionUpdate,
    ResolvedChain, WebChainEditor, WebPickerCatalog,
    active_search_section_override, default_chain_for_backend, ensure_web_search_config_coherence,
    ensure_web_search_config_coherence_at, migrate_legacy_search_override,
    persist_search_backend_as_chain, search_override_warning, search_section_override_from_path,
    capability_label, chain_summary_after_save, clear_web_search_chain_in_config,
    clear_web_search_section_overrides, clear_web_section_overrides, collect_web_diagnostics, format_extract_doctor_detail,
    format_picker_label, format_search_chain_summary, format_search_doctor_detail,
    format_web_setup_report, get_web_search_backend, list_web_search_backends,
    load_web_search_config_from_disk, load_web_search_config_from_path,
    persist_search_chain_order, persist_search_chain_with_timeout,
    persist_web_backend_in_config, persist_web_search_chain_in_config,
    persist_web_section_in_config, print_provider_detail_cli, provider_detail_lines,
    register_web_search_backend, render_web_dashboard, reset_web_to_auto,
    web_command_overlay, web_command_usage, web_menu_status_hint, web_provider_picker_rows,
    web_search_is_available, web_status_one_liner,
};
