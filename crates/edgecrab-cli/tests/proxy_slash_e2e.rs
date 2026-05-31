//! E2E: `/proxy` slash registration (non-TTY).

use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn slash_proxy_enable_grok_writes_config() {
    let home = tempdir().expect("home");
    let config_dir = home.path().join(".edgecrab");
    fs::create_dir_all(&config_dir).expect("dir");
    let config_path = config_dir.join("config.yaml");
    fs::write(
        &config_path,
        r#"
proxy:
  bind: 127.0.0.1
  port: 11434
"#,
    )
    .expect("write");

    // Simulate slash handler path: `enable grok` via CLI mirror (same hub code as /proxy enable grok).
    let output = Command::new(env!("CARGO_BIN_EXE_edgecrab"))
        .arg("--config")
        .arg(&config_path)
        .args(["proxy", "enable", "grok"])
        .env("HOME", home.path())
        .output()
        .expect("run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let saved = fs::read_to_string(&config_path).expect("read");
    assert!(saved.contains("xai"));
    assert!(saved.contains("grok"));
}
