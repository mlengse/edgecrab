//! Shared `/computer` diagnostics and polished status reports (CLI + gateway + TUI).

use std::process::Command;

use crate::toolsets::{contains_all_sentinel, COMPUTER_USE_TOOLS};

use super::permissions::{cua_driver_binary_available, install_hint, is_macos};
use super::vision_routing::{active_provider_model, should_route_capture_to_aux_vision};

const SCREEN_RECORDING_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";
const ACCESSIBILITY_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadinessMark {
    Ok,
    Warn,
    Fail,
}

impl ReadinessMark {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "[ok]",
            Self::Warn => "[warn]",
            Self::Fail => "[fail]",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputerUseStatusConfig {
    pub enabled: bool,
    pub keep_last_n_screenshots: u32,
    pub confirm_destructive: bool,
    pub cua_driver_cmd: String,
}

#[derive(Debug, Clone, Default)]
pub struct ComputerUseReportContext {
    pub active_model: String,
    pub enabled_toolsets: Vec<String>,
    pub disabled_toolsets: Vec<String>,
    pub auxiliary_provider: Option<String>,
    pub auxiliary_model: Option<String>,
    pub auxiliary_base_url: Option<String>,
    pub noop_backend: bool,
}

impl ComputerUseReportContext {
    pub fn from_app_config_ref(cfg: &crate::config_ref::AppConfigRef) -> Self {
        Self {
            active_model: cfg.active_model.clone(),
            enabled_toolsets: cfg.parent_active_toolsets.clone(),
            disabled_toolsets: cfg.disabled_toolsets.clone(),
            auxiliary_provider: cfg.auxiliary_provider.clone(),
            auxiliary_model: cfg.auxiliary_model.clone(),
            auxiliary_base_url: cfg.auxiliary_base_url.clone(),
            noop_backend: std::env::var("EDGECRAB_COMPUTER_USE_BACKEND")
                .map(|v| v.eq_ignore_ascii_case("noop"))
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputerUseSnapshot {
    pub platform_supported: bool,
    pub driver_installed: bool,
    pub driver_cmd: String,
    pub config_enabled: bool,
    pub toolset_active: bool,
    pub accessibility: Option<crate::macos_permissions::MacosConsentState>,
    pub vision_route: VisionRouteSummary,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionRouteSummary {
    MultimodalMain,
    AuxiliaryVision,
    TextOnlyAx,
    Unavailable,
}

impl VisionRouteSummary {
    fn label(self) -> &'static str {
        match self {
            Self::MultimodalMain => "multimodal (main model handles screenshots)",
            Self::AuxiliaryVision => "auxiliary vision pre-analysis",
            Self::TextOnlyAx => "text-only (AX/SOM index; no PNG path)",
            Self::Unavailable => "unavailable on this platform",
        }
    }
}

pub fn is_computer_use_toolset_active(
    enabled_toolsets: &[String],
    disabled_toolsets: &[String],
) -> bool {
    if disabled_toolsets
        .iter()
        .any(|name| name.eq_ignore_ascii_case("computer_use"))
    {
        return false;
    }
    enabled_toolsets.is_empty()
        || contains_all_sentinel(enabled_toolsets)
        || enabled_toolsets
            .iter()
            .any(|name| name.eq_ignore_ascii_case("computer_use"))
}

pub fn collect_snapshot(
    cfg: &ComputerUseStatusConfig,
    ctx: &ComputerUseReportContext,
) -> ComputerUseSnapshot {
    let platform_supported = is_macos() || ctx.noop_backend;
    let driver_installed = ctx.noop_backend || cua_driver_binary_available(&cfg.cua_driver_cmd);
    let toolset_active = is_computer_use_toolset_active(&ctx.enabled_toolsets, &ctx.disabled_toolsets);
    let accessibility = if is_macos() {
        crate::macos_permissions::accessibility_consent_status()
    } else {
        None
    };
    let vision_route = vision_route_summary(cfg, ctx);
    let ready = platform_supported
        && driver_installed
        && cfg.enabled
        && toolset_active
        && (ctx.noop_backend
            || accessibility == Some(crate::macos_permissions::MacosConsentState::Granted));

    ComputerUseSnapshot {
        platform_supported,
        driver_installed,
        driver_cmd: cfg.cua_driver_cmd.clone(),
        config_enabled: cfg.enabled,
        toolset_active,
        accessibility,
        vision_route,
        ready,
    }
}

fn vision_route_summary(cfg: &ComputerUseStatusConfig, ctx: &ComputerUseReportContext) -> VisionRouteSummary {
    if !is_macos() && !ctx.noop_backend {
        return VisionRouteSummary::Unavailable;
    }
    let app_cfg = crate::config_ref::AppConfigRef {
        active_model: ctx.active_model.clone(),
        auxiliary_provider: ctx.auxiliary_provider.clone(),
        auxiliary_model: ctx.auxiliary_model.clone(),
        auxiliary_base_url: ctx.auxiliary_base_url.clone(),
        ..Default::default()
    };
    let (provider, model) = active_provider_model(&app_cfg);
    if should_route_capture_to_aux_vision(&provider, &model, &app_cfg) {
        VisionRouteSummary::AuxiliaryVision
    } else if super::permissions::check_requirements(&cfg.cua_driver_cmd) || ctx.noop_backend {
        VisionRouteSummary::MultimodalMain
    } else {
        VisionRouteSummary::TextOnlyAx
    }
}

pub fn computer_command_usage() -> &'static str {
    "Computer use (macOS + cua-driver):\n\
     /computer, /computer status   — readiness report\n\
     /computer permissions         — permission + driver checklist\n\
     /computer open                — open Screen Recording + Accessibility settings\n\
     /computer enable | disable    — persist config + toolset\n\
     /computer help                — show this help"
}

pub fn format_computer_command(
    sub: &str,
    cfg: &ComputerUseStatusConfig,
    ctx: &ComputerUseReportContext,
) -> String {
    let sub = sub.trim().to_ascii_lowercase();
    match sub.as_str() {
        "" | "status" => render_status_report(cfg, ctx),
        "permissions" => render_permissions_report(cfg, ctx),
        "help" => computer_command_usage().into(),
        "open" => render_open_settings_report(cfg, ctx),
        other => format!(
            "Unknown /computer subcommand '{other}'.\n\n{}",
            computer_command_usage()
        ),
    }
}

/// Overlay-friendly title/subtitle/body for the TUI report panel.
pub fn computer_command_overlay(
    sub: &str,
    cfg: &ComputerUseStatusConfig,
    ctx: &ComputerUseReportContext,
) -> (String, String, String) {
    let sub = sub.trim().to_ascii_lowercase();
    let snapshot = collect_snapshot(cfg, ctx);
    let subtitle = if snapshot.ready {
        "Ready for background desktop control".into()
    } else {
        "Setup required before computer_use is available".into()
    };
    let (title, body) = match sub.as_str() {
        "" | "status" => ("Computer Use".into(), render_status_report(cfg, ctx)),
        "permissions" => (
            "Computer Use — Permissions".into(),
            render_permissions_report(cfg, ctx),
        ),
        "help" => ("Computer Use — Help".into(), computer_command_usage().into()),
        "open" => (
            "Computer Use — Settings".into(),
            render_open_settings_report(cfg, ctx),
        ),
        other => (
            "Computer Use".into(),
            format!(
                "Unknown /computer subcommand '{other}'.\n\n{}",
                computer_command_usage()
            ),
        ),
    };
    (title, subtitle, body)
}

pub fn format_computer_enable_result(enabled: bool, saved: bool, error: Option<&str>) -> String {
    if let Some(err) = error {
        return format!("Failed to persist computer_use.enabled: {err}");
    }
    if !saved {
        return "computer_use setting unchanged.".into();
    }
    if enabled {
        "computer_use enabled and added to enabled_toolsets.\n\
         Grant Screen Recording + Accessibility (/computer open), then run /computer status."
            .into()
    } else {
        "computer_use disabled in config.yaml.".into()
    }
}

fn render_status_report(cfg: &ComputerUseStatusConfig, ctx: &ComputerUseReportContext) -> String {
    let snap = collect_snapshot(cfg, ctx);
    let mut lines = vec![
        "Computer Use — macOS background desktop control (cua-driver)".into(),
        String::new(),
        "Readiness".into(),
        line(
            "Platform",
            if snap.platform_supported {
                ReadinessMark::Ok
            } else {
                ReadinessMark::Fail
            },
            if snap.platform_supported {
                "macOS (or EDGECRAB_COMPUTER_USE_BACKEND=noop for tests)".into()
            } else {
                "macOS required".into()
            },
        ),
        line(
            "cua-driver",
            if snap.driver_installed {
                ReadinessMark::Ok
            } else {
                ReadinessMark::Fail
            },
            if ctx.noop_backend {
                "noop test backend".into()
            } else if snap.driver_installed {
                format!("found ({})", snap.driver_cmd)
            } else {
                format!("not found ({})", snap.driver_cmd)
            },
        ),
        line(
            "Config enabled",
            mark_bool(snap.config_enabled),
            if snap.config_enabled {
                "computer_use.enabled: true".into()
            } else {
                "computer_use.enabled: false — run /computer enable".into()
            },
        ),
        line(
            "Toolset active",
            mark_bool(snap.toolset_active),
            if snap.toolset_active {
                format!("{} registered", COMPUTER_USE_TOOLS.join(", "))
            } else {
                "add computer_use to enabled_toolsets".into()
            },
        ),
        line(
            "Accessibility",
            accessibility_mark(snap.accessibility),
            accessibility_detail(snap.accessibility),
        ),
        line(
            "Screen Recording",
            ReadinessMark::Warn,
            "no public preflight API — verify in System Settings".into(),
        ),
        String::new(),
        "Configuration".into(),
        format!("  keep_last_n_screenshots: {}", cfg.keep_last_n_screenshots),
        format!("  confirm_destructive: {}", cfg.confirm_destructive),
        format!(
            "  active_model: {}",
            if ctx.active_model.is_empty() {
                "(session not started)".into()
            } else {
                ctx.active_model.clone()
            }
        ),
        format!("  capture routing: {}", snap.vision_route.label()),
        String::new(),
        "Overall".into(),
        if snap.ready {
            "  Status: READY — computer_use can drive the desktop in the background.".into()
        } else {
            "  Status: NOT READY — complete the items marked [fail] or [warn] above.".into()
        },
    ];
    lines.extend(next_steps(&snap));
    lines.join("\n")
}

fn render_permissions_report(cfg: &ComputerUseStatusConfig, ctx: &ComputerUseReportContext) -> String {
    let snap = collect_snapshot(cfg, ctx);
    let mut lines = vec![
        "Computer Use — permission checklist".into(),
        String::new(),
        "Required on macOS:".into(),
        "  1. cua-driver on PATH (install via /computer help)".into(),
        "  2. Screen Recording for EdgeCrab / cua-driver / terminal host".into(),
        "  3. Accessibility for EdgeCrab / cua-driver / terminal host".into(),
        String::new(),
        line(
            "cua-driver",
            if snap.driver_installed {
                ReadinessMark::Ok
            } else {
                ReadinessMark::Fail
            },
            if snap.driver_installed {
                "installed".into()
            } else {
                "missing".into()
            },
        ),
        line(
            "Accessibility",
            accessibility_mark(snap.accessibility),
            accessibility_detail(snap.accessibility),
        ),
        line(
            "Screen Recording",
            ReadinessMark::Warn,
            "open System Settings → Privacy & Security → Screen Recording".into(),
        ),
    ];
    if !snap.driver_installed && !ctx.noop_backend {
        lines.push(String::new());
        lines.push("Install cua-driver:".into());
        lines.push(install_hint().into());
    }
    lines.push(String::new());
    lines.push("Tip: run `/computer open` to jump to the privacy panes.".into());
    lines.join("\n")
}

fn render_open_settings_report(cfg: &ComputerUseStatusConfig, ctx: &ComputerUseReportContext) -> String {
    let mut notes = open_computer_use_settings();
    let mut body = render_permissions_report(cfg, ctx);
    if !notes.is_empty() {
        body.push_str("\n\nActions:\n");
        for note in notes.drain(..) {
            body.push_str(&format!("- {note}\n"));
        }
    }
    body
}

pub fn open_computer_use_settings() -> Vec<String> {
    if !is_macos() {
        return vec!["Screen Recording and Accessibility settings are macOS-only.".into()];
    }
    let mut notes = Vec::new();
    for (label, url) in [
        ("Screen Recording", SCREEN_RECORDING_SETTINGS_URL),
        ("Accessibility", ACCESSIBILITY_SETTINGS_URL),
    ] {
        match Command::new("open").arg(url).status() {
            Ok(status) if status.success() => {
                notes.push(format!("Opened {label} privacy settings."));
            }
            Ok(status) => {
                notes.push(format!("Opening {label} settings returned exit status {}.", status));
            }
            Err(err) => {
                notes.push(format!("Could not open {label} settings: {err}"));
            }
        }
    }
    notes
}

fn next_steps(snap: &ComputerUseSnapshot) -> Vec<String> {
    if snap.ready {
        return vec![
            String::new(),
            "Next steps".into(),
            "  • Ask the agent to capture an app: computer_use action=capture app=Safari".into(),
            "  • Use element indices from capture for click/type actions".into(),
            "  • Run /computer permissions if capture fails with a privacy error".into(),
        ];
    }
    let mut steps = vec![String::new(), "Next steps".into()];
    if !snap.driver_installed {
        steps.push("  1. Install cua-driver (see /computer permissions)".into());
    }
    if !snap.config_enabled {
        steps.push("  • Run /computer enable".into());
    }
    if !snap.toolset_active {
        steps.push("  • Add computer_use to enabled_toolsets in config.yaml".into());
    }
    if snap.accessibility != Some(crate::macos_permissions::MacosConsentState::Granted) {
        steps.push("  • Run /computer open and grant Accessibility + Screen Recording".into());
    }
    steps
}

fn line(label: &str, mark: ReadinessMark, detail: String) -> String {
    format!("  {mark} {label:<18} {detail}", mark = mark.label())
}

fn mark_bool(value: bool) -> ReadinessMark {
    if value {
        ReadinessMark::Ok
    } else {
        ReadinessMark::Warn
    }
}

fn accessibility_mark(state: Option<crate::macos_permissions::MacosConsentState>) -> ReadinessMark {
    match state {
        Some(crate::macos_permissions::MacosConsentState::Granted) => ReadinessMark::Ok,
        Some(crate::macos_permissions::MacosConsentState::Denied) => ReadinessMark::Fail,
        Some(crate::macos_permissions::MacosConsentState::WouldPrompt) => ReadinessMark::Warn,
        Some(crate::macos_permissions::MacosConsentState::Unknown) | None => ReadinessMark::Warn,
    }
}

fn accessibility_detail(state: Option<crate::macos_permissions::MacosConsentState>) -> String {
    match state {
        Some(crate::macos_permissions::MacosConsentState::Granted) => "granted".into(),
        Some(crate::macos_permissions::MacosConsentState::Denied) => {
            "denied — reset in System Settings or tccutil".into()
        }
        Some(crate::macos_permissions::MacosConsentState::WouldPrompt) => {
            "not yet granted (macOS will prompt on first use)".into()
        }
        Some(crate::macos_permissions::MacosConsentState::Unknown) | None => {
            if is_macos() {
                "unknown — grant via /computer open".into()
            } else {
                "n/a".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cfg() -> ComputerUseStatusConfig {
        ComputerUseStatusConfig {
            enabled: false,
            keep_last_n_screenshots: 3,
            confirm_destructive: true,
            cua_driver_cmd: "cua-driver".into(),
        }
    }

    #[test]
    fn toolset_active_when_explicitly_enabled() {
        assert!(is_computer_use_toolset_active(
            &["computer_use".into()],
            &[]
        ));
    }

    #[test]
    fn toolset_inactive_when_disabled() {
        assert!(!is_computer_use_toolset_active(
            &["computer_use".into()],
            &["computer_use".into()]
        ));
    }

    #[test]
    fn status_report_mentions_readiness_sections() {
        let body = render_status_report(
            &sample_cfg(),
            &ComputerUseReportContext {
                enabled_toolsets: vec!["computer_use".into()],
                ..Default::default()
            },
        );
        assert!(body.contains("Readiness"));
        assert!(body.contains("Configuration"));
        assert!(body.contains("NOT READY"));
    }

    #[test]
    fn help_lists_enable_and_open() {
        let text = computer_command_usage();
        assert!(text.contains("/computer enable"));
        assert!(text.contains("/computer open"));
    }

    #[test]
    fn overlay_titles_are_human_readable() {
        let (title, subtitle, _) = computer_command_overlay(
            "status",
            &sample_cfg(),
            &ComputerUseReportContext::default(),
        );
        assert_eq!(title, "Computer Use");
        assert!(subtitle.contains("Setup"));
    }
}
