//! macOS availability probe for computer use.

pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

pub fn cua_driver_binary_available(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

pub fn install_hint() -> &'static str {
    "cua-driver is not installed. Install with:\n\
     /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/trycua/cua/main/libs/cua-driver/scripts/install.sh)\"\n\
     Then grant Screen Recording + Accessibility in System Settings → Privacy & Security."
}

pub fn permissions_status(cmd: &str) -> String {
    if !is_macos() {
        return "computer_use: macOS only (current platform unsupported)".into();
    }
    if !cua_driver_binary_available(cmd) {
        return format!("computer_use: cua-driver not found ({cmd})\n{}\n", install_hint());
    }
    format!(
        "computer_use: cua-driver found ({cmd})\n\
         Ensure Screen Recording and Accessibility are granted for EdgeCrab / cua-driver."
    )
}

pub fn check_requirements(cmd: &str) -> bool {
    is_macos() && cua_driver_binary_available(cmd)
}
