//! E2E: `edgecrab proxy` CLI (grok / xAI presets, non-interactive).

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn edgecrab(home: &std::path::Path, config_path: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_edgecrab"));
    cmd.arg("--config").arg(config_path).env("HOME", home);
    cmd
}

fn minimal_proxy_config() -> &'static str {
    r#"
proxy:
  bind: 127.0.0.1
  port: 11434
"#
}

#[test]
fn proxy_overview_prints_grok_quick_start() {
    let home = tempdir().expect("home");
    let config_dir = home.path().join(".edgecrab");
    fs::create_dir_all(&config_dir).expect("dir");
    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, minimal_proxy_config()).expect("write");

    let output = edgecrab(home.path(), &config_path)
        .args(["proxy"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("setup grok"));
    assert!(stdout.contains("start --provider xai"));
}

#[test]
fn proxy_enable_grok_writes_xai_upstream_and_alias() {
    let home = tempdir().expect("home");
    let config_dir = home.path().join(".edgecrab");
    fs::create_dir_all(&config_dir).expect("dir");
    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, minimal_proxy_config()).expect("write");

    let output = edgecrab(home.path(), &config_path)
        .args(["proxy", "enable", "grok"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("xAI") || stdout.contains("Grok"));
    assert!(stdout.contains("forward:xai") || stdout.contains("grok"));

    let saved = fs::read_to_string(&config_path).expect("read config");
    assert!(saved.contains("xai"), "expected xai upstream in config:\n{saved}");
    assert!(saved.contains("grok"), "expected grok alias in config:\n{saved}");
}

#[test]
fn proxy_setup_grok_yes_is_non_interactive() {
    let home = tempdir().expect("home");
    let config_dir = home.path().join(".edgecrab");
    fs::create_dir_all(&config_dir).expect("dir");
    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, minimal_proxy_config()).expect("write");

    let output = edgecrab(home.path(), &config_path)
        .args(["proxy", "setup", "grok", "--yes"])
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("grok") || stdout.contains("xAI"));
    assert!(stdout.contains("OPENAI_API_BASE") || stdout.contains("Client configuration"));

    let token_path = config_dir.join("proxy-token");
    assert!(token_path.exists(), "proxy token should be created");

    let saved = fs::read_to_string(&config_path).expect("read config");
    assert!(saved.contains("forward:xai") || saved.contains("xai:"));
}

#[test]
fn proxy_doctor_and_client_after_enable_grok() {
    let home = tempdir().expect("home");
    let config_dir = home.path().join(".edgecrab");
    fs::create_dir_all(&config_dir).expect("dir");
    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, minimal_proxy_config()).expect("write");

    let enable = edgecrab(home.path(), &config_path)
        .args(["proxy", "enable", "grok"])
        .output()
        .expect("enable");
    assert!(enable.status.success());

    let doctor = edgecrab(home.path(), &config_path)
        .args(["proxy", "doctor"])
        .output()
        .expect("doctor");
    assert!(doctor.status.success());
    let doctor_out = String::from_utf8_lossy(&doctor.stdout);
    assert!(doctor_out.contains("doctor"));

    let token = edgecrab(home.path(), &config_path)
        .args(["proxy", "token", "set", "cli-e2e-token"])
        .output()
        .expect("token set");
    assert!(token.status.success());

    let client = edgecrab(home.path(), &config_path)
        .args(["proxy", "client", "--show-token"])
        .output()
        .expect("client");
    assert!(client.status.success());
    let client_out = String::from_utf8_lossy(&client.stdout);
    assert!(client_out.contains("11434"));
    assert!(client_out.contains("grok") || client_out.contains("OPENAI_API_BASE"));
    assert!(client_out.contains("cli-e2e-token"));
}
