//! TUI-friendly web search/extract reports — `/web` slash command overlays.
//!
//! Mirrors the computer-use overlay pattern: one-liner for the chat pane, rich
//! dashboard for the full-screen document overlay.

use super::provider_diagnostics::{
    WebDiagnosticsReport, WebProviderStatus, capability_label, collect_web_diagnostics,
    format_extract_doctor_detail, format_search_doctor_detail,
};

const CAP_LEGEND: &str = "Capabilities: S=search  E=extract  C=crawl";

fn provider_state_icon(p: &WebProviderStatus) -> &'static str {
    if p.configured && p.available {
        "✓"
    } else if !p.missing_env.is_empty() {
        "·"
    } else if p.configured {
        "⚠"
    } else if p.id == "ddgs" {
        "○"
    } else {
        "·"
    }
}

fn provider_state_label(p: &WebProviderStatus) -> &'static str {
    if p.configured && p.available {
        "ready"
    } else if !p.missing_env.is_empty() {
        "needs key"
    } else if p.configured {
        "configured"
    } else if p.id == "ddgs" {
        "no key"
    } else {
        "optional"
    }
}

fn readiness_badge(ready: bool) -> &'static str {
    if ready {
        "✓ READY"
    } else {
        "✗ NOT READY"
    }
}

fn extract_readiness_badge(report: &WebDiagnosticsReport) -> &'static str {
    if report.paid_extract_configured || report.configured_extract_override.is_some() {
        "✓ READY"
    } else {
        "○ native"
    }
}

/// One-line status for the TUI output pane when opening `/web`.
pub fn web_status_one_liner() -> String {
    let report = collect_web_diagnostics();
    web_status_one_liner_from(&report)
}

pub fn web_status_one_liner_from(report: &WebDiagnosticsReport) -> String {
    if report.search_ready {
        format!("🔍 Web ready — chain: {}", report.search_chain_summary)
    } else {
        format!(
            "🔍 Web needs a backend — run /web and press Enter on one with ✓ (or add keys to ~/.edgecrab/.env)"
        )
    }
}

/// Short hint for the interactive `/web` hub menu header.
pub fn web_menu_status_hint(report: &WebDiagnosticsReport) -> String {
    if report.search_ready {
        format!(
            "search ready · chain: {}",
            report.search_chain_summary
        )
    } else {
        "search not ready — run Setup wizard".into()
    }
}

/// Short help for `/web help`.
pub fn web_command_usage() -> String {
    r#"WEB SEARCH — /web

  /web              Open configurator (pick primary + fallbacks)
  /web help         This message

In the configurator
  Enter     Set primary backend (saves immediately)
  Space     Toggle fallback on/off
  a         Reset to auto
  r         Refresh status
  Esc       Close

API keys: ~/.edgecrab/.env (BRAVE_API_KEY, SEARXNG_URL, TAVILY_API_KEY, …)
ddgs needs no key but may be blocked — set a paid backend as primary.
"#
    .to_string()
}

fn render_at_a_glance(report: &WebDiagnosticsReport) -> String {
    let mut out = String::from(" AT A GLANCE\n ───────────\n");
    out.push_str(&format!(
        "  Search   {}  {}\n",
        readiness_badge(report.search_ready),
        format_search_doctor_detail(report)
    ));
    out.push_str(&format!(
        "  Extract  {}  {}\n",
        extract_readiness_badge(report),
        format_extract_doctor_detail(report)
    ));
    out.push_str(&format!(
        "  Chain    {}  ({}s timeout)\n",
        report.search_chain_summary, report.search_chain_timeout_secs
    ));
    if let Some(ref s) = report.configured_search_override {
        out.push_str(&format!("  Config   web.search_backend = {s}\n"));
    } else if let Some(ref e) = report.configured_extract_override {
        out.push_str(&format!("  Config   web.extract_backend = {e}\n"));
    } else if let Some(ref b) = report.resolved_search_backend {
        out.push_str(&format!("  Config   web.backend = {b}\n"));
    }
    out
}

fn render_provider_row(p: &WebProviderStatus) -> String {
    let caps = capability_label(p.supports_search, p.supports_extract, p.supports_crawl);
    let mut line = format!(
        "  {} {:<12} [{:<3}] {:<10} {}\n",
        provider_state_icon(p),
        p.id,
        caps,
        provider_state_label(p),
        p.display_name
    );
    if !p.missing_env.is_empty() {
        line.push_str(&format!("      └─ set: {}\n", p.missing_env.join(", ")));
    }
    line
}

fn render_providers_grouped(report: &WebDiagnosticsReport) -> String {
    let mut ready = Vec::new();
    let mut optional = Vec::new();
    let mut needs_key = Vec::new();

    for p in &report.providers {
        if p.configured && p.available {
            ready.push(p);
        } else if p.missing_env.is_empty() || p.id == "ddgs" {
            optional.push(p);
        } else {
            needs_key.push(p);
        }
    }

    let mut out = String::from("\n PROVIDERS\n ─────────\n");
    out.push_str(&format!("  {CAP_LEGEND}\n\n"));

    if !ready.is_empty() {
        out.push_str(&format!("  ACTIVE ({})\n", ready.len()));
        for p in ready {
            out.push_str(&render_provider_row(p));
        }
        out.push('\n');
    }

    if !optional.is_empty() {
        out.push_str(&format!("  NO KEY REQUIRED ({})\n", optional.len()));
        for p in optional {
            out.push_str(&render_provider_row(p));
        }
        out.push('\n');
    }

    if !needs_key.is_empty() {
        out.push_str(&format!("  NEEDS API KEY ({})\n", needs_key.len()));
        for p in needs_key {
            out.push_str(&render_provider_row(p));
        }
    }

    out
}

fn render_next_steps(report: &WebDiagnosticsReport) -> String {
    let mut steps = Vec::new();
    if !report.search_ready {
        steps.push("Open `/web` → Setup wizard to configure a search backend.");
    } else {
        let unconfigured_paid: Vec<_> = report
            .providers
            .iter()
            .filter(|p| p.supports_search && !p.missing_env.is_empty() && !(p.configured && p.available))
            .map(|p| p.id.as_str())
            .take(3)
            .collect();
        if !unconfigured_paid.is_empty() {
            steps.push(
                "Optional: add API keys in ~/.edgecrab/.env for higher-quality search (firecrawl, brave, …).",
            );
        }
    }
    if steps.is_empty() {
        return String::from("\n ✓ All set — web_search and web_extract are ready.\n");
    }
    let mut out = String::from("\n NEXT STEPS\n ──────────\n");
    for (i, step) in steps.iter().enumerate() {
        out.push_str(&format!("  {}. {step}\n", i + 1));
    }
    out
}

fn render_quick_actions_footer() -> String {
    r#"
 QUICK NAV (from this overlay)
 ─────────────────────────────
  s setup   c chain   d doctor   p providers   h help   r refresh   b hub
"#
    .to_string()
}

/// Full dashboard body for `/web` and `/web status`.
pub fn render_web_dashboard() -> String {
    render_web_dashboard_from(&collect_web_diagnostics())
}

pub fn render_web_dashboard_from(report: &WebDiagnosticsReport) -> String {
    let mut out = String::from(
        "╔══════════════════════════════════════════════════════╗\n\
         ║  WEB SEARCH & EXTRACT                                ║\n\
         ╚══════════════════════════════════════════════════════╝\n",
    );
    out.push_str(&render_at_a_glance(report));
    out.push_str(&render_providers_grouped(report));
    out.push_str(&render_next_steps(report));
    out.push_str(&render_quick_actions_footer());
    out
}

fn chain_backends(report: &WebDiagnosticsReport) -> Vec<String> {
    let mut backends = Vec::new();
    if let Some(ref primary) = report.search_chain_primary {
        backends.push(primary.clone());
    }
    backends.extend(report.search_chain_fallbacks.clone());
    backends
}

fn render_chain_visual(report: &WebDiagnosticsReport) -> String {
    let backends = chain_backends(report);
    let mut out = String::from(" FALLBACK FLOW\n ─────────────\n\n");

    if backends.is_empty() {
        out.push_str(&format!(
            "  (auto)  {}\n\n",
            report.search_chain_summary
        ));
    } else {
        for (i, name) in backends.iter().enumerate() {
            let step = i + 1;
            out.push_str(&format!("  [{step}] {name}"));
            if i + 1 < backends.len() {
                out.push_str("\n        │\n        ▼\n");
            } else {
                out.push('\n');
            }
        }
        out.push('\n');
    }

    let override_note = if report.configured_search_override.is_some()
        || report.resolved_search_backend.is_some()
    {
        "\n  ⚠ web.search_backend / web.backend overrides this chain when set.\n"
    } else {
        ""
    };

    out.push_str(&format!(
        " CONFIG\n ──────\n\
         Summary:   {}\n\
         Timeout:   {}s\n\
         Primary:   {}\n\
         Fallbacks: {}{override_note}\n\
         \n\
         HOW IT WORKS\n\
         ────────────\n\
         1. Try primary (skips unconfigured paid backends automatically)\n\
         2. On timeout, 429, or 5xx → next fallback\n\
         3. ddgs is the no-key terminal fallback\n\
         \n\
         Change chain: /web setup → Configure search fallback chain\n\
         \n\
         Quick nav: s setup   r refresh   Esc close",
        report.search_chain_summary,
        report.search_chain_timeout_secs,
        report.search_chain_primary.as_deref().unwrap_or("(auto)"),
        if report.search_chain_fallbacks.is_empty() {
            "(none)".to_string()
        } else {
            report.search_chain_fallbacks.join(", ")
        },
    ));
    out
}

fn render_doctor_detail(report: &WebDiagnosticsReport) -> String {
    let mut out = String::from(
        " TECHNICAL DIAGNOSTICS\n\
         ────────────────────\n\n",
    );
    out.push_str(&format!(
        "  Search ready:     {}\n\
         Search detail:    {}\n\
         Extract detail:   {}\n\
         Resolved search:  {}\n\
         Resolved extract: {}\n\
         Chain:            {}\n",
        if report.search_ready { "yes" } else { "no" },
        format_search_doctor_detail(report),
        format_extract_doctor_detail(report),
        report
            .resolved_search_backend
            .as_deref()
            .unwrap_or("(auto chain)"),
        report
            .resolved_extract_backend
            .as_deref()
            .unwrap_or("(native + paid APIs)"),
        report.search_chain_summary,
    ));

    if report.configured_search_override.is_some()
        || report.configured_extract_override.is_some()
        || report.resolved_search_backend.is_some()
    {
        out.push_str("\n CONFIG OVERRIDES\n ────────────────\n");
        if let Some(ref s) = report.configured_search_override {
            out.push_str(&format!("  web.search_backend = {s}\n"));
        }
        if let Some(ref e) = report.configured_extract_override {
            out.push_str(&format!("  web.extract_backend = {e}\n"));
        }
        if let Some(ref b) = report.resolved_search_backend {
            out.push_str(&format!("  web.backend = {b}\n"));
        }
    }

    out.push_str("\n PROVIDER CHECK\n ──────────────\n");
    for p in &report.providers {
        let caps = capability_label(p.supports_search, p.supports_extract, p.supports_crawl);
        out.push_str(&format!(
            "  {:<12} [{:<3}] configured={} available={}\n",
            p.id, caps, p.configured, p.available
        ));
        if !p.missing_env.is_empty() {
            out.push_str(&format!("               missing: {}\n", p.missing_env.join(", ")));
        }
    }

    out.push_str("\n Quick nav: s setup   p providers   r refresh   Esc close");
    out
}

fn render_providers_only(report: &WebDiagnosticsReport) -> String {
    let mut out = String::from(" REGISTERED BACKENDS\n ───────────────────\n");
    out.push_str(&format!("  {CAP_LEGEND}\n"));
    out.push_str(&render_providers_grouped(report));
    out.push_str("\n Quick nav: s setup   d doctor   h help   Esc close");
    out
}

fn overlay_subtitle(sub: &str, report: &WebDiagnosticsReport) -> String {
    match sub {
        "chain" | "fallback" | "fallbacks" => report.search_chain_summary.clone(),
        "doctor" => format!(
            "search: {} · extract: {}",
            if report.search_ready { "ok" } else { "needs setup" },
            if report.paid_extract_configured {
                "paid API"
            } else {
                "native"
            }
        ),
        "setup" => "Configure backends, keys, and chain".into(),
        "providers" | "list" => {
            let ready = report
                .providers
                .iter()
                .filter(|p| p.configured && p.available)
                .count();
            format!("{ready}/{} providers ready", report.providers.len())
        }
        "help" => "Commands · shortcuts · tips".into(),
        _ => web_menu_status_hint(report),
    }
}

/// Overlay title/subtitle/body triple for the TUI document panel.
pub fn web_command_overlay(sub: &str) -> (String, String, String) {
    let sub = sub.trim().to_ascii_lowercase();
    let first = sub.split_whitespace().next().unwrap_or("status");
    let report = collect_web_diagnostics();
    web_command_overlay_from(first, &report)
}

pub fn web_command_overlay_from(
    sub: &str,
    report: &WebDiagnosticsReport,
) -> (String, String, String) {
    let subtitle = overlay_subtitle(sub, report);
    match sub {
        "chain" | "fallback" | "fallbacks" => (
            "Web — Fallback Chain".into(),
            subtitle,
            render_chain_visual(report),
        ),
        "doctor" => (
            "Web — Diagnostics".into(),
            subtitle,
            render_doctor_detail(report),
        ),
        "providers" | "list" => (
            "Web — Providers".into(),
            subtitle,
            render_providers_only(report),
        ),
        "help" => (
            "Web — Help".into(),
            subtitle,
            web_command_usage(),
        ),
        _ => (
            "Web Search & Extract".into(),
            subtitle,
            render_web_dashboard_from(report),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::super::registry::{reset_registry_for_tests, test_registry_lock};
    use super::*;

    #[test]
    fn one_liner_mentions_search_state() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let line = web_status_one_liner();
        assert!(line.starts_with("🔍 Web"));
    }

    #[test]
    fn dashboard_has_grouped_providers() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let text = render_web_dashboard();
        assert!(text.contains("AT A GLANCE"));
        assert!(text.contains("PROVIDERS"));
        assert!(text.contains("ddgs"));
        assert!(text.contains("s setup"));
    }

    #[test]
    fn doctor_does_not_duplicate_full_dashboard() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let report = collect_web_diagnostics();
        let (_, _, body) = web_command_overlay_from("doctor", &report);
        assert!(body.contains("TECHNICAL DIAGNOSTICS"));
        assert!(!body.contains("AT A GLANCE"));
    }

    #[test]
    fn chain_view_has_flow_section() {
        let _lock = test_registry_lock();
        reset_registry_for_tests();
        let (_, _, body) = web_command_overlay("chain");
        assert!(body.contains("FALLBACK FLOW"));
    }

    #[test]
    fn help_lists_shortcuts() {
        let usage = web_command_usage();
        assert!(usage.contains("/web"));
        assert!(usage.contains("Enter"));
        assert!(usage.contains("Space"));
    }
}
