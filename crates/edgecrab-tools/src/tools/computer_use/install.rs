//! cua-driver install / upgrade (Hermes-aligned upstream install.sh).

use std::process::Command;

pub const CUA_DRIVER_INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/trycua/cua/main/libs/cua-driver/scripts/install.sh";

pub const CUA_DRIVER_INSTALL_SHELL: &str = "/bin/bash -c \"$(curl -fsSL \
    https://raw.githubusercontent.com/trycua/cua/main/libs/cua-driver/scripts/install.sh)\"";

/// cua-driver release EdgeCrab targets — mirrors Hermes'
/// `PINNED_CUA_DRIVER_VERSION` (default `0.5.0`).
///
/// Note: Hermes leaves this as a decorative constant — its installer runs the
/// bare upstream `install.sh` (baked default) and exports `HERMES_CUA_DRIVER_VERSION`,
/// which `install.sh` never reads (the script reads `CUA_DRIVER_VERSION`).
/// EdgeCrab instead wires the pin into `install.sh` via `CUA_DRIVER_VERSION`
/// (see [`run_installer`]) with a retry-to-latest fallback so a pin that is not
/// yet published upstream cannot break installs.
pub const PINNED_CUA_DRIVER_VERSION: &str = "0.5.0";

/// Pure resolver for the cua-driver version pin given the two override env values.
///
/// Precedence: `EDGECRAB_CUA_DRIVER_VERSION` → `HERMES_CUA_DRIVER_VERSION`
/// (honored for hermes-agent migrants) → [`PINNED_CUA_DRIVER_VERSION`].
/// Blank/whitespace overrides are ignored.
fn resolve_pinned_version(edgecrab: Option<&str>, hermes: Option<&str>) -> String {
    edgecrab
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .or_else(|| hermes.map(str::trim).filter(|v| !v.is_empty()))
        .map(str::to_string)
        .unwrap_or_else(|| PINNED_CUA_DRIVER_VERSION.to_string())
}

/// True when the resolved pin came from an explicit env override (vs. the
/// built-in default). Explicit overrides are always passed to `install.sh`;
/// the default is attempted first but falls back to latest if unpublished.
fn has_explicit_version_override(edgecrab: Option<&str>, hermes: Option<&str>) -> bool {
    [edgecrab, hermes]
        .into_iter()
        .flatten()
        .any(|v| !v.trim().is_empty())
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// The cua-driver version EdgeCrab targets, honoring env overrides.
pub fn pinned_cua_driver_version() -> String {
    resolve_pinned_version(
        env_opt("EDGECRAB_CUA_DRIVER_VERSION").as_deref(),
        env_opt("HERMES_CUA_DRIVER_VERSION").as_deref(),
    )
}

fn pinned_version_is_explicit() -> bool {
    has_explicit_version_override(
        env_opt("EDGECRAB_CUA_DRIVER_VERSION").as_deref(),
        env_opt("HERMES_CUA_DRIVER_VERSION").as_deref(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed,
    AlreadyInstalled,
    Upgraded,
    Failed,
    SkippedNonMacos,
}

#[derive(Debug, Clone)]
pub struct CuaDriverInstallResult {
    pub outcome: InstallOutcome,
    pub messages: Vec<String>,
    pub path_before: Option<String>,
    pub path_after: Option<String>,
    pub version_before: Option<String>,
    pub version_after: Option<String>,
}

impl CuaDriverInstallResult {
    pub fn ok(&self) -> bool {
        matches!(
            self.outcome,
            InstallOutcome::Installed | InstallOutcome::AlreadyInstalled | InstallOutcome::Upgraded
        )
    }
}

pub fn resolve_driver_path(cmd: &str) -> Option<String> {
    which::which(cmd)
        .ok()
        .map(|p| p.display().to_string())
}

pub fn driver_version(cmd: &str) -> Option<String> {
    Command::new(cmd)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Parse `/computer install`, `/computer install upgrade`, `/computer upgrade`.
pub fn parse_install_args(args: &str) -> (bool, bool) {
    let lower = args.trim().to_ascii_lowercase();
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    let wants_install = tokens.first().is_some_and(|t| *t == "install" || *t == "upgrade");
    let upgrade = tokens.first().is_some_and(|t| *t == "upgrade")
        || tokens.iter().any(|t| *t == "upgrade" || *t == "--upgrade");
    (wants_install, upgrade)
}

pub fn install_cua_driver(cmd: &str, upgrade: bool) -> CuaDriverInstallResult {
    let mut messages = Vec::new();

    if !super::permissions::is_macos() {
        messages.push("Computer Use (cua-driver) is macOS-only.".into());
        return CuaDriverInstallResult {
            outcome: InstallOutcome::SkippedNonMacos,
            messages,
            path_before: None,
            path_after: None,
            version_before: None,
            version_after: None,
        };
    }

    let path_before = resolve_driver_path(cmd);
    let version_before = path_before.as_ref().and_then(|_| driver_version(cmd));

    if path_before.is_some() && !upgrade {
        let version = version_before
            .clone()
            .unwrap_or_else(|| "unknown version".into());
        messages.push(format!("{cmd} already installed: {version}"));
        if let Some(ref path) = path_before {
            messages.push(format!("  path: {path}"));
        }
        messages.push("Grant macOS permissions if not done yet:".into());
        messages.push("  System Settings → Privacy & Security → Accessibility".into());
        messages.push("  System Settings → Privacy & Security → Screen Recording".into());
        messages.push("Run `/computer open` to jump there, then `/computer status`.".into());
        messages.push("Refresh to latest: `/computer install upgrade`".into());
        return CuaDriverInstallResult {
            outcome: InstallOutcome::AlreadyInstalled,
            messages,
            path_before: path_before.clone(),
            path_after: path_before,
            version_before: version_before.clone(),
            version_after: version_before,
        };
    }

    if which::which("curl").is_err() {
        messages.push("curl not found — install curl or run manually:".into());
        messages.push(format!("  {CUA_DRIVER_INSTALL_SHELL}"));
        messages.push("Docs: https://github.com/trycua/cua/tree/main/libs/cua-driver".into());
        return CuaDriverInstallResult {
            outcome: InstallOutcome::Failed,
            messages,
            path_before,
            path_after: None,
            version_before,
            version_after: None,
        };
    }

    if !check_cua_driver_asset_for_arch(&mut messages) {
        return CuaDriverInstallResult {
            outcome: InstallOutcome::Failed,
            messages,
            path_before: path_before.clone(),
            path_after: path_before,
            version_before,
            version_after: None,
        };
    }

    let label = if upgrade { "Refreshing" } else { "Installing" };
    messages.push(format!("{label} cua-driver (macOS background computer-use)…"));
    messages.push(format!(
        "Target version: {} (override via EDGECRAB_CUA_DRIVER_VERSION).",
        pinned_cua_driver_version()
    ));
    messages.push("This downloads the matching release from GitHub (may take a minute).".into());

    match run_installer() {
        Ok(()) => {
            let path_after = resolve_driver_path(cmd);
            let version_after = path_after.as_ref().and_then(|_| driver_version(cmd));
            if path_after.is_some() {
                let outcome = if upgrade && path_before.is_some() {
                    InstallOutcome::Upgraded
                } else {
                    InstallOutcome::Installed
                };
                messages.push(format!("{cmd} installed successfully."));
                if let Some(ref path) = path_after {
                    messages.push(format!("  path: {path}"));
                }
                if let (Some(before), Some(after)) = (&version_before, &version_after) {
                    if before != after {
                        messages.push(format!("  version: {before} → {after}"));
                    } else if !after.is_empty() {
                        messages.push(format!("  version: {after} (up to date)"));
                    }
                } else if let Some(ref ver) = version_after {
                    messages.push(format!("  version: {ver}"));
                }
                messages.push(String::new());
                messages.push("IMPORTANT — grant macOS permissions now:".into());
                messages.push("  1. Run `/computer open`".into());
                messages.push("  2. Enable Accessibility + Screen Recording for:".into());
                messages.push("     • your terminal (Terminal.app / iTerm / Cursor)".into());
                messages.push("     • EdgeCrab (this app)".into());
                messages.push("  3. Run `/computer enable` then `/computer status`".into());
                CuaDriverInstallResult {
                    outcome,
                    messages,
                    path_before,
                    path_after,
                    version_before,
                    version_after,
                }
            } else {
                messages.push(format!(
                    "Installer finished but `{cmd}` is still not on PATH."
                ));
                messages.push("Restart your shell or add ~/.local/bin to PATH, then `/computer status`.".into());
                messages.push(format!("Manual install: {CUA_DRIVER_INSTALL_SHELL}"));
                CuaDriverInstallResult {
                    outcome: InstallOutcome::Failed,
                    messages,
                    path_before,
                    path_after: None,
                    version_before,
                    version_after: None,
                }
            }
        }
        Err(err) => {
            messages.push(format!("Install failed: {err}"));
            messages.push(format!("Re-run manually: {CUA_DRIVER_INSTALL_SHELL}"));
            CuaDriverInstallResult {
                outcome: InstallOutcome::Failed,
                messages,
                path_before: path_before.clone(),
                path_after: path_before,
                version_before,
                version_after: None,
            }
        }
    }
}

fn run_installer() -> Result<(), String> {
    let pinned = pinned_cua_driver_version();
    let explicit = pinned_version_is_explicit();

    // Attempt the pinned version first (same release Hermes targets). The
    // upstream install.sh reads CUA_DRIVER_VERSION and resolves the matching
    // `cua-driver-v<ver>` release tag.
    match run_install_script(Some(&pinned)) {
        Ok(()) => Ok(()),
        Err(pinned_err) => {
            if explicit {
                // The user asked for this exact version — don't silently swap it.
                Err(pinned_err)
            } else {
                // Default pin may not be published upstream yet — fall back to
                // the installer's baked latest so installs never hard-fail.
                tracing::warn!(
                    "cua-driver pin {pinned} install failed ({pinned_err}); retrying latest"
                );
                run_install_script(None)
            }
        }
    }
}

/// Run the upstream `install.sh`. When `version` is `Some`, the matching release
/// tag is pinned via `CUA_DRIVER_VERSION`; otherwise the script's baked latest
/// is installed.
fn run_install_script(version: Option<&str>) -> Result<(), String> {
    let mut cmd = Command::new("/bin/bash");
    cmd.arg("-c").arg(format!(
        "curl -fsSL {CUA_DRIVER_INSTALL_SCRIPT_URL} | /bin/bash"
    ));
    if let Some(version) = version {
        cmd.env("CUA_DRIVER_VERSION", version);
    }

    let status = cmd.status().map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("installer exited with {status}"))
    }
}

/// Intel Macs may lack a published asset — warn before attempting install.
fn check_cua_driver_asset_for_arch(messages: &mut Vec<String>) -> bool {
    if cfg!(target_arch = "aarch64") {
        return true;
    }

    let output = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "Accept: application/vnd.github+json",
            "https://api.github.com/repos/trycua/cua/releases/latest",
        ])
        .output();

    let Ok(output) = output else {
        return true;
    };
    if !output.status.success() {
        return true;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let tag = extract_json_string(&body, "tag_name").unwrap_or_default();
    let has_intel = body.contains("x86_64") || body.contains("amd64");
    if !has_intel && !tag.is_empty() {
        messages.push(format!(
            "Latest CUA release ({tag}) has no Intel (x86_64) asset."
        ));
        messages.push("cua-driver currently ships Apple Silicon builds only.".into());
        messages.push("See: https://github.com/trycua/cua/issues/1493".into());
        return false;
    }
    true
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = json.find(&needle)?;
    let after_key = &json[start + needle.len()..];
    let colon = after_key.find(':')?;
    let rest = after_key[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

pub fn render_install_report(result: &CuaDriverInstallResult) -> String {
    let mut lines = vec![
        "Computer Use — cua-driver install".into(),
        String::new(),
        match result.outcome {
            InstallOutcome::Installed => "Result: installed".into(),
            InstallOutcome::AlreadyInstalled => "Result: already installed".into(),
            InstallOutcome::Upgraded => "Result: upgraded".into(),
            InstallOutcome::Failed => "Result: failed".into(),
            InstallOutcome::SkippedNonMacos => "Result: skipped (macOS only)".into(),
        },
        String::new(),
    ];
    for msg in &result.messages {
        if msg.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(format!("  {msg}"));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_install_upgrade_variants() {
        assert_eq!(parse_install_args("install"), (true, false));
        assert_eq!(parse_install_args("install upgrade"), (true, true));
        assert_eq!(parse_install_args("install --upgrade"), (true, true));
        assert_eq!(parse_install_args("upgrade"), (true, true));
        assert_eq!(parse_install_args("status"), (false, false));
    }

    #[test]
    fn install_report_renders_outcome() {
        let report = render_install_report(&CuaDriverInstallResult {
            outcome: InstallOutcome::AlreadyInstalled,
            messages: vec!["cua-driver already installed".into()],
            path_before: Some("/usr/local/bin/cua-driver".into()),
            path_after: Some("/usr/local/bin/cua-driver".into()),
            version_before: Some("1.0.0".into()),
            version_after: Some("1.0.0".into()),
        });
        assert!(report.contains("already installed"));
        assert!(report.contains("cua-driver already installed"));
    }

    #[test]
    fn pin_defaults_to_hermes_target() {
        assert_eq!(resolve_pinned_version(None, None), PINNED_CUA_DRIVER_VERSION);
        assert_eq!(resolve_pinned_version(Some("  "), Some("")), PINNED_CUA_DRIVER_VERSION);
    }

    #[test]
    fn pin_honors_edgecrab_override_first() {
        assert_eq!(
            resolve_pinned_version(Some("0.3.1"), Some("0.5.0")),
            "0.3.1"
        );
    }

    #[test]
    fn pin_falls_back_to_hermes_env() {
        assert_eq!(resolve_pinned_version(None, Some("0.4.2")), "0.4.2");
        assert_eq!(resolve_pinned_version(Some("   "), Some("0.4.2")), "0.4.2");
    }

    #[test]
    fn explicit_override_detection() {
        assert!(!has_explicit_version_override(None, None));
        assert!(!has_explicit_version_override(Some("  "), Some("")));
        assert!(has_explicit_version_override(Some("0.2.0"), None));
        assert!(has_explicit_version_override(None, Some("0.5.0")));
    }
}
