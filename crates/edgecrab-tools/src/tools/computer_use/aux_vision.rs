//! Route computer_use captures through auxiliary vision (Hermes #24015).

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;

use super::backend::{CaptureResult, UIElement};
use super::response::cache_dir;
use crate::registry::ToolContext;
use crate::tools::vision::analyze_local_image;

fn element_to_json(e: &UIElement) -> serde_json::Value {
    json!({
        "index": e.index,
        "role": e.role,
        "label": e.label,
        "bounds": [e.bounds.0, e.bounds.1, e.bounds.2, e.bounds.3],
        "app": e.app,
    })
}

/// Pre-analyse a capture PNG via auxiliary vision; return text-only JSON on success.
pub async fn route_capture_through_aux_vision(
    cap: &CaptureResult,
    summary: &str,
    ctx: &ToolContext,
) -> Option<String> {
    let b64 = cap.png_b64.as_ref()?;
    let raw = STANDARD.decode(b64).ok()?;
    let ext = if b64.starts_with("/9j/") { ".jpg" } else { ".png" };
    let dir = cache_dir(&ctx.config.edgecrab_home);
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("aux_{}{ext}", uuid::Uuid::new_v4()));
    std::fs::write(&path, raw).ok()?;

    let prompt = format!(
        "Describe what is visible in this macOS application screenshot in \
         concise but specific terms. Mention the app name and window title if \
         visible, the overall layout, any labelled buttons, menus or text fields, \
         and any prominent text content the user would need to know about. Do not \
         invent details that are not actually visible.\n\n\
         AX/SOM index for cross-reference:\n{summary}"
    );

    let result = analyze_local_image(ctx, &path, &prompt).await.ok()?;
    let _ = std::fs::remove_file(&path);

    let analysis = result.analysis.trim();
    if analysis.is_empty() {
        return None;
    }

    Some(
        serde_json::to_string(&json!({
            "mode": cap.mode,
            "width": cap.width,
            "height": cap.height,
            "app": cap.app,
            "window_title": cap.window_title,
            "elements": cap.elements.iter().map(element_to_json).collect::<Vec<_>>(),
            "summary": summary,
            "vision_analysis": analysis,
            "vision_analysis_routed_via": "auxiliary.vision",
        }))
        .unwrap_or_default(),
    )
}
