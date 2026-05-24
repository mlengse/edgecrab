//! Shared `/computer` status formatting (CLI + gateway).

use super::permissions::permissions_status;

#[derive(Debug, Clone)]
pub struct ComputerUseStatusConfig {
    pub enabled: bool,
    pub keep_last_n_screenshots: u32,
    pub confirm_destructive: bool,
    pub cua_driver_cmd: String,
}

pub fn format_computer_command(sub: &str, cfg: &ComputerUseStatusConfig) -> String {
    let sub = sub.trim().to_ascii_lowercase();
    let cmd = &cfg.cua_driver_cmd;
    match sub.as_str() {
        "" | "status" => format!(
            "computer_use.enabled: {}\nkeep_last_n_screenshots: {}\nconfirm_destructive: {}\ncua_driver_cmd: {cmd}\n\n{}",
            cfg.enabled,
            cfg.keep_last_n_screenshots,
            cfg.confirm_destructive,
            permissions_status(cmd),
        ),
        "permissions" => permissions_status(cmd),
        other => format!("Unknown /computer subcommand '{other}'. Use: status | permissions"),
    }
}
