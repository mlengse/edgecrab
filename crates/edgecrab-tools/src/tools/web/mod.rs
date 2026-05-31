//! Web tools — pluggable search, extract, and crawl.

mod extract_crawl;
pub mod search;

pub use extract_crawl::{WebCrawlTool, WebExtractTool};
pub use search::{
    SearchResult, WebSearchBackend, WebSearchChainUpdate, WebSearchTool, WebSectionUpdate,
    ResolvedChain,
    capability_label, clear_web_search_chain_in_config, clear_web_section_overrides,
    collect_web_diagnostics, format_extract_doctor_detail, format_search_chain_summary,
    format_search_doctor_detail, format_web_setup_report, get_web_search_backend,
    list_web_search_backends, load_web_search_config_from_disk, load_web_search_config_from_path,
    persist_web_backend_in_config, persist_web_search_chain_in_config,
    persist_web_section_in_config, register_web_search_backend, render_web_dashboard,
    web_command_overlay, web_command_usage, web_menu_status_hint, web_provider_picker_rows,
    web_search_is_available, web_status_one_liner,
};
