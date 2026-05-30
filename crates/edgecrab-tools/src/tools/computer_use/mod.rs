//! Provider-agnostic macOS desktop control via cua-driver MCP.
//!
//! Mirrors Hermes `tools/computer_use/`. macOS + cua-driver required for live use;
//! `EDGECRAB_COMPUTER_USE_BACKEND=noop` forces the test stub.

mod aux_vision;
mod backend;
mod browsers;
mod cua_backend;
mod dispatch;
mod install;
#[cfg(test)]
mod manual_e2e;
mod mcp;
mod noop;
mod permissions;
mod response;
mod safety;
mod schema;
mod status;
mod text_input;
pub mod guidance;
mod vision_routing;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use serde_json::json;

use edgecrab_types::{ToolError, ToolSchema};

use crate::registry::{ToolContext, ToolHandler};

pub use permissions::{check_requirements, permissions_status};
pub use response::parse_multimodal_tool_output;
pub use install::{install_cua_driver, parse_install_args, render_install_report, CuaDriverInstallResult};
pub use guidance::{COMPUTER_USE_GUIDANCE_COMPACT, COMPUTER_USE_GUIDANCE_FULL};
pub use status::{
    ComputerUseReportContext, ComputerUseStatusConfig, collect_snapshot, computer_command_overlay,
    computer_status_one_liner,
    computer_command_usage, format_computer_command, format_computer_enable_result,
    format_computer_setup_report, is_computer_use_toolset_active, open_computer_use_settings,
};
pub use vision_routing::{
    provider_accepts_multimodal_tool_result, should_route_capture_to_aux_vision,
};

mod backend_pool;

#[cfg(test)]
pub async fn reset_backend_for_tests() {
    backend_pool::reset_pool_for_tests().await;
}

pub struct ComputerUseTool;

#[async_trait]
impl ToolHandler for ComputerUseTool {
    fn name(&self) -> &'static str {
        "computer_use"
    }

    fn toolset(&self) -> &'static str {
        "computer_use"
    }

    fn emoji(&self) -> &'static str {
        "🖥️"
    }

    fn schema(&self) -> ToolSchema {
        schema::computer_use_schema()
    }

    fn check_fn(&self, ctx: &ToolContext) -> bool {
        if !ctx.config.computer_use_enabled {
            return false;
        }
        if std::env::var("EDGECRAB_COMPUTER_USE_BACKEND")
            .map(|v| v.eq_ignore_ascii_case("noop"))
            .unwrap_or(false)
        {
            return true;
        }
        permissions::check_requirements(&ctx.config.computer_use_cua_cmd)
    }

    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String, ToolError> {
        if !ctx.config.computer_use_enabled {
            return Err(ToolError::PermissionDenied(
                "computer_use is disabled. Set computer_use.enabled: true in config.yaml.".into(),
            ));
        }

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "computer_use".into(),
                message: "missing `action`".into(),
            })?
            .to_ascii_lowercase();

        if action == "type"
            && let Some(text) = args.get("text").and_then(|v| v.as_str())
            && let Some(pat) = safety::blocked_type_pattern(text)
        {
            return Ok(json!({
                "error": format!("blocked pattern in type text: {pat:?}"),
                "hint": "Dangerous shell patterns cannot be typed via computer_use."
            })
            .to_string());
        }
        if action == "key"
            && let Some(keys) = args.get("keys").and_then(|v| v.as_str())
            && let Some(blocked) = safety::blocked_key_combo(keys)
        {
            return Ok(json!({
                "error": format!("blocked key combo: {blocked:?}"),
                "hint": "Destructive system shortcuts are hard-blocked."
            })
            .to_string());
        }

        safety::ensure_destructive_approved(ctx, &action, &args).await?;

        let home = ctx.config.edgecrab_home.clone();
        let cmd = ctx.config.computer_use_cua_cmd.clone();
        let session_id = ctx.session_id.clone();
        let action_owned = action;
        let args_owned = args;

        let handle = backend_pool::session_handle(&session_id, &cmd)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "computer_use".into(),
                message: e,
            })?;
        let mut backend = handle.lock().await;
        dispatch::dispatch_action(
            backend.as_mut(),
            &action_owned,
            &args_owned,
            &home,
            Some(ctx),
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "computer_use".into(),
            message: e,
        })
    }
}

inventory::submit!(&ComputerUseTool as &dyn ToolHandler);
