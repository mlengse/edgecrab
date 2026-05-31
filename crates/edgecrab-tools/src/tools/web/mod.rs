//! Web tools — pluggable search, extract, and crawl.

mod extract_crawl;
pub mod search;

pub use extract_crawl::{WebCrawlTool, WebExtractTool};
pub use search::{
    ResolvedChain, SearchResult, WebChainEditor, WebPickerCatalog, WebSearchBackend,
    WebSearchChainUpdate, WebSearchTool, WebSectionUpdate, active_search_section_override,
    capability_label, chain_summary_after_save, clear_web_search_chain_in_config,
    clear_web_search_section_overrides, clear_web_section_overrides, collect_web_diagnostics,
    default_chain_for_backend, effective_web_search_config, ensure_web_search_config_coherence,
    ensure_web_search_config_coherence_at, format_extract_doctor_detail, format_picker_label,
    format_saved_chain_summary, format_search_chain_summary, format_search_doctor_detail,
    format_web_search_result_count, format_web_search_status_line, format_web_setup_report,
    gateway_web_command_reply, get_web_search_backend, list_web_search_backends,
    load_web_search_config_from_disk, load_web_search_config_from_path,
    migrate_legacy_search_override, persist_search_backend_as_chain, persist_search_chain_order,
    persist_search_chain_with_timeout, persist_web_backend_in_config,
    persist_web_search_chain_in_config, persist_web_section_in_config, print_provider_detail_cli,
    provider_detail_lines, register_web_search_backend, render_web_dashboard, reset_web_to_auto,
    search_override_warning, search_section_override_from_path, summarize_web_search_backend,
    web_command_overlay, web_command_usage, web_menu_status_hint, web_provider_picker_rows,
    web_search_is_available, web_search_result_note, web_status_one_liner,
};
