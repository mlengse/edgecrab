//! Manual macOS e2e checks — run with cua-driver installed and permissions granted.
//!
//! ```bash
//! cargo test -p edgecrab-tools manual_e2e -- --ignored --nocapture
//! ```

#[cfg(test)]
mod tests {
    use crate::registry::{ToolContext, ToolHandler};
    use crate::tools::computer_use::ComputerUseTool;
    use serde_json::json;

    fn live_ctx() -> ToolContext {
        // SAFETY: test-only env override for backend selection.
        unsafe { std::env::remove_var("EDGECRAB_COMPUTER_USE_BACKEND") };
        let mut ctx = ToolContext::test_context();
        ctx.config.computer_use_enabled = true;
        ctx.config.computer_use_cua_cmd = "cua-driver".into();
        ctx
    }

    #[tokio::test]
    #[ignore = "requires macOS, cua-driver, Screen Recording, and Accessibility"]
    async fn live_capture_returns_multimodal_or_ax_payload() {
        if !cfg!(target_os = "macos") {
            return;
        }
        let tool = ComputerUseTool;
        let out = tool
            .execute(json!({ "action": "capture", "mode": "som" }), &live_ctx())
            .await
            .expect("capture");
        assert!(
            out.contains("capture mode=") || out.contains("_multimodal"),
            "unexpected capture payload: {out}"
        );
    }
}
