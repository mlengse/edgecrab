//! Shared `/computer` diagnostics and polished status reports (CLI + gateway + TUI).

use std::process::Command;

use crate::toolsets::{contains_all_sentinel, COMPUTER_USE_TOOLS};

use super::install::{
    install_cua_driver, render_install_report, CuaDriverInstallResult, CUA_DRIVER_INSTALL_SHELL,
};
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
    "Computer Use — macOS background desktop control (cua-driver)\n\
     \n\
     Setup (first time)\n\
       /computer setup              guided wizard: install → enable → open settings\n\
       /computer install            download & install cua-driver from GitHub\n\
       /computer install upgrade    refresh to latest cua-driver release\n\
       /computer enable             persist computer_use.enabled + toolset\n\
       /computer open               open Accessibility + Screen Recording panes\n\
       /computer status             readiness checklist (run until READY)\n\
     \n\
     Diagnostics\n\
       /computer permissions        permission + driver checklist\n\
       /computer disable            turn off computer_use\n\
       /computer help               show this help\n\
     \n\
     Using computer_use with the agent (once READY)\n\
       Ask naturally — the agent calls the computer_use tool:\n\
         \"Capture Safari and tell me what's on screen\"\n\
         \"Click the address bar and go to example.com\"\n\
     \n\
       Or reference explicit actions:\n\
         capture  — screenshot + accessibility tree with element indices\n\
         click    — click element by index from last capture\n\
         type     — type text into focused field\n\
         key      — press keys (Return, cmd+c, etc.)\n\
         scroll   — scroll in a direction\n\
     \n\
       Example workflow:\n\
         1. capture app=Safari          → see elements [0] [1] [2]…\n\
         2. click element=2             → focus search bar\n\
         3. type text=\"edgecrab.dev\"   → enter URL\n\
         4. key keys=Return             → navigate\n\
         5. capture app=Safari          → verify result\n\
     \n\
     Tip: text-only models still get AX/SOM indices; multimodal models\n\
     also receive screenshots. Run /computer status to see capture routing."
}

pub fn format_computer_command(
    sub: &str,
    cfg: &ComputerUseStatusConfig,
    ctx: &ComputerUseReportContext,
) -> String {
    let sub = sub.trim().to_ascii_lowercase();
    let (wants_install, upgrade) = super::install::parse_install_args(&sub);
    if wants_install {
        let result = install_cua_driver(&cfg.cua_driver_cmd, upgrade);
        return render_install_report(&result);
    }
    match sub.as_str() {
        "" | "status" => render_status_report(cfg, ctx),
        "permissions" => render_permissions_report(cfg, ctx),
        "help" => computer_command_usage().into(),
        "open" => render_open_settings_report(cfg, ctx),
        "setup" => {
            let install = install_cua_driver(&cfg.cua_driver_cmd, false);
            render_setup_report(cfg, ctx, &install, None, &[])
        }
        other => format!(
            "Unknown /computer subcommand '{other}'.\n\n{}",
            computer_command_usage()
        ),
    }
}

/// One-line status for the TUI output pane (shown when opening `/computer` reports).
pub fn computer_status_one_liner(
    cfg: &ComputerUseStatusConfig,
    ctx: &ComputerUseReportContext,
) -> String {
    let snap = collect_snapshot(cfg, ctx);
    if snap.ready {
        return format!(
            "🖥 Computer Use: READY — {} · capture: {}",
            COMPUTER_USE_TOOLS.join(", "),
            snap.vision_route.label()
        );
    }
    let mut blockers = Vec::new();
    if !snap.platform_supported {
        blockers.push("macOS required");
    }
    if !snap.driver_installed {
        blockers.push("cua-driver missing");
    }
    if !snap.config_enabled {
        blockers.push("disabled in config");
    }
    if !snap.toolset_active {
        blockers.push("toolset inactive");
    }
    if !ctx.noop_backend
        && snap.accessibility != Some(crate::macos_permissions::MacosConsentState::Granted)
    {
        blockers.push("Accessibility not granted");
    }
    let detail = if blockers.is_empty() {
        "check permissions".into()
    } else {
        blockers.join(", ")
    };
    format!(
        "🖥 Computer Use: NOT READY — {detail}. Try `/computer setup` or open the report with Esc."
    )
}

fn overlay_subtitle(sub: &str, snap: &ComputerUseSnapshot) -> String {
    match sub {
        "permissions" => {
            if snap.driver_installed
                && snap.accessibility == Some(crate::macos_permissions::MacosConsentState::Granted)
            {
                "Driver installed · Accessibility granted".into()
            } else {
                "Driver + Accessibility + Screen Recording".into()
            }
        }
        "open" => "Opened System Settings privacy panes".into(),
        "help" => "Slash commands · setup · agent examples".into(),
        "setup" => "Install → enable → permissions → checklist".into(),
        s if super::install::parse_install_args(s).0 => "cua-driver from GitHub (trycua/cua)".into(),
        _ if snap.ready => format!(
            "READY — {} · {}",
            COMPUTER_USE_TOOLS.join(", "),
            snap.vision_route.label()
        ),
        _ => {
            let mut n = 0u8;
            if !snap.platform_supported {
                n += 1;
            }
            if !snap.driver_installed {
                n += 1;
            }
            if !snap.config_enabled {
                n += 1;
            }
            if !snap.toolset_active {
                n += 1;
            }
            if snap.accessibility != Some(crate::macos_permissions::MacosConsentState::Granted) {
                n += 1;
            }
            format!("NOT READY — {n} checklist item(s) need attention")
        }
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
        "setup" => (
            "Computer Use — Setup Wizard".into(),
            {
                let install = install_cua_driver(&cfg.cua_driver_cmd, false);
                render_setup_report(cfg, ctx, &install, None, &[])
            },
        ),
        other if super::install::parse_install_args(other).0 => {
            let (_, upgrade) = super::install::parse_install_args(other);
            let result = install_cua_driver(&cfg.cua_driver_cmd, upgrade);
            (
                if upgrade {
                    "Computer Use — Upgrade".into()
                } else {
                    "Computer Use — Install".into()
                },
                render_install_report(&result),
            )
        }
        other => (
            "Computer Use".into(),
            format!(
                "Unknown /computer subcommand '{other}'.\n\n{}",
                computer_command_usage()
            ),
        ),
    };
    let subtitle = overlay_subtitle(&sub, &snapshot);
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
         Next: /computer open → grant permissions → /computer status"
            .into()
    } else {
        "computer_use disabled in config.yaml.".into()
    }
}

/// Combined setup wizard report (install + enable + open settings).
pub fn format_computer_setup_report(
    cfg: &ComputerUseStatusConfig,
    ctx: &ComputerUseReportContext,
    install: &CuaDriverInstallResult,
    enable_saved: Option<bool>,
    open_notes: &[String],
) -> String {
    render_setup_report(cfg, ctx, install, enable_saved, open_notes)
}

fn render_setup_report(
    cfg: &ComputerUseStatusConfig,
    ctx: &ComputerUseReportContext,
    install: &CuaDriverInstallResult,
    enable_saved: Option<bool>,
    open_notes: &[String],
) -> String {
    let mut lines = vec![
        "Computer Use — guided setup".into(),
        String::new(),
        "This wizard runs the recommended first-time flow:".into(),
        "  1. Install cua-driver".into(),
        "  2. Enable computer_use in config".into(),
        "  3. Open macOS privacy settings".into(),
        "  4. Show readiness checklist".into(),
        String::new(),
        "Step 1 — cua-driver".into(),
    ];
    for msg in &install.messages {
        if msg.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(format!("  {msg}"));
        }
    }

    lines.push(String::new());
    lines.push("Step 2 — enable computer_use".into());
    match enable_saved {
        Some(true) => lines.push("  [ok] computer_use.enabled saved to config.yaml".into()),
        Some(false) => lines.push("  [warn] enable skipped or already enabled".into()),
        None => lines.push("  [ ] run `/computer enable` (not executed in preview)".into()),
    }

    lines.push(String::new());
    lines.push("Step 3 — macOS permissions".into());
    if open_notes.is_empty() {
        lines.push("  [ ] run `/computer open` to jump to Accessibility + Screen Recording".into());
        lines.push("  Grant both for your terminal host AND EdgeCrab.".into());
    } else {
        for note in open_notes {
            lines.push(format!("  • {note}"));
        }
    }

    lines.push(String::new());
    lines.push("Step 4 — readiness".into());
    let status = render_status_report(cfg, ctx);
    for line in status.lines().skip(1) {
        lines.push(format!("  {line}"));
    }

    if !install.ok() {
        lines.push(String::new());
        lines.push("Install did not complete — fix Step 1, then re-run `/computer setup`.".into());
    } else if enable_saved != Some(true) {
        lines.push(String::new());
        lines.push("Next: `/computer enable` → `/computer open` → `/computer status`".into());
    } else {
        lines.push(String::new());
        lines.push("Next: grant permissions in System Settings, then `/computer status` until READY.".into());
    }
    lines.join("\n")
}

fn driver_detail(snap: &ComputerUseSnapshot, ctx: &ComputerUseReportContext) -> String {
    if ctx.noop_backend {
        return "noop test backend".into();
    }
    if !snap.driver_installed {
        return format!(
            "not found — run `/computer install`\n       or: {CUA_DRIVER_INSTALL_SHELL}"
        );
    }
    let path = super::install::resolve_driver_path(&snap.driver_cmd);
    let version = super::install::driver_version(&snap.driver_cmd);
    match (path, version) {
        (Some(p), Some(v)) => format!("{v} at {p}"),
        (Some(p), None) => format!("found at {p}"),
        (None, Some(v)) => format!("{v} ({})", snap.driver_cmd),
        (None, None) => format!("found ({})", snap.driver_cmd),
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
            driver_detail(&snap, ctx),
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
        "  1. cua-driver on PATH — `/computer install`".into(),
        "  2. Screen Recording for terminal host + EdgeCrab + cua-driver".into(),
        "  3. Accessibility for terminal host + EdgeCrab + cua-driver".into(),
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
        lines.push("  Run `/computer install` in EdgeCrab (recommended)".into());
        lines.push(format!("  Or manually: {CUA_DRIVER_INSTALL_SHELL}"));
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
            "Quick start — ask the agent:".into(),
            "  \"Capture Safari and describe what's on screen\"".into(),
            "  \"Click element 3 and type hello world\"".into(),
            String::new(),
            "Tool actions (computer_use):".into(),
            "  capture app=Safari  → screenshot + element indices".into(),
            "  click element=3     → click indexed element".into(),
            "  type text=\"…\"       → type into focused field".into(),
            "  key keys=Return     → press keys".into(),
            String::new(),
            "Run `/computer help` for the full workflow and examples.".into(),
        ];
    }
    let mut steps = vec![
        String::new(),
        "Next steps — run in order:".into(),
    ];
    let mut n = 1;
    if !snap.driver_installed {
        steps.push(format!("  {n}. `/computer install`  — download cua-driver"));
        n += 1;
    }
    if !snap.config_enabled {
        steps.push(format!("  {n}. `/computer enable`   — turn on in config"));
        n += 1;
    }
    if !snap.toolset_active {
        steps.push(format!(
            "  {n}. Add computer_use to enabled_toolsets in config.yaml"
        ));
        n += 1;
    }
    if snap.accessibility != Some(crate::macos_permissions::MacosConsentState::Granted) {
        steps.push(format!(
            "  {n}. `/computer open`    — grant Accessibility + Screen Recording"
        ));
        n += 1;
    }
    steps.push(format!("  {n}. `/computer status`   — repeat until READY"));
    steps.push(String::new());
    steps.push("Shortcut: `/computer setup` runs install + enable + open in one go.".into());
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
            "denied — System Settings → Privacy → Accessibility; \
             enable Terminal/iTerm/Cursor + EdgeCrab; or: tccutil reset Accessibility"
                .into()
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
    fn help_lists_install_setup_and_examples() {
        let text = computer_command_usage();
        assert!(text.contains("/computer install"));
        assert!(text.contains("/computer setup"));
        assert!(text.contains("capture app=Safari"));
    }

    #[test]
    fn next_steps_suggest_install_when_driver_missing() {
        let snap = ComputerUseSnapshot {
            platform_supported: true,
            driver_installed: false,
            driver_cmd: "cua-driver".into(),
            config_enabled: true,
            toolset_active: true,
            accessibility: Some(crate::macos_permissions::MacosConsentState::Denied),
            vision_route: VisionRouteSummary::TextOnlyAx,
            ready: false,
        };
        let steps = next_steps(&snap);
        let joined = steps.join("\n");
        assert!(joined.contains("/computer install"));
        assert!(joined.contains("/computer setup"));
    }

    #[test]
    fn overlay_titles_are_human_readable() {
        let (title, subtitle, _) = computer_command_overlay(
            "status",
            &sample_cfg(),
            &ComputerUseReportContext::default(),
        );
        assert_eq!(title, "Computer Use");
        assert!(subtitle.contains("NOT READY"));
    }

    #[test]
    fn status_one_liner_mentions_ready_or_blockers() {
        let not_ready = computer_status_one_liner(
            &sample_cfg(),
            &ComputerUseReportContext {
                enabled_toolsets: vec!["computer_use".into()],
                ..Default::default()
            },
        );
        assert!(not_ready.contains("NOT READY"));

        let mut cfg = sample_cfg();
        cfg.enabled = true;
        let ready = computer_status_one_liner(
            &cfg,
            &ComputerUseReportContext {
                enabled_toolsets: vec!["computer_use".into()],
                noop_backend: true,
                ..Default::default()
            },
        );
        assert!(ready.contains("READY"));
    }
}
