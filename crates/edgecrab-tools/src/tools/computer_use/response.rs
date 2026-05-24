//! Response shaping + screenshot cache (mirrors Hermes `tool.py` response helpers).

use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;

use super::backend::{ActionResult, CaptureResult, UIElement};
use super::schema::{coerce_max_elements, DEFAULT_MAX_ELEMENTS};

pub fn cache_dir(edgecrab_home: &Path) -> PathBuf {
    edgecrab_home.join("cache").join("computer_use")
}

pub fn save_screenshot_png(edgecrab_home: &Path, png_b64: &str) -> Result<PathBuf, String> {
    let dir = cache_dir(edgecrab_home);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4();
    let path = dir.join(format!("{id}.png"));
    let raw = STANDARD.decode(png_b64).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())?;
    Ok(path)
}

fn element_to_json(e: &UIElement) -> serde_json::Value {
    json!({
        "index": e.index,
        "role": e.role,
        "label": e.label,
        "bounds": [e.bounds.0, e.bounds.1, e.bounds.2, e.bounds.3],
        "app": e.app,
    })
}

fn format_elements(elements: &[UIElement], max_lines: usize) -> Vec<String> {
    let mut out = Vec::new();
    for e in elements.iter().take(max_lines) {
        let label = e.label.replace('\n', " ");
        let label = if label.len() > 60 {
            format!("{}…", &label[..60])
        } else {
            label
        };
        out.push(format!(
            "  #{} {} {:?} @ {:?}",
            e.index, e.role, label, e.bounds
        ));
    }
    if elements.len() > max_lines {
        out.push(format!(
            "  ... +{} more (call capture with app= to narrow)",
            elements.len() - max_lines
        ));
    }
    out
}

pub fn capture_response(
    cap: &CaptureResult,
    max_elements: u32,
    edgecrab_home: &Path,
) -> Result<String, String> {
    let max_elements = if max_elements == 0 {
        DEFAULT_MAX_ELEMENTS
    } else {
        max_elements
    };
    let total = cap.elements.len();
    let visible: Vec<_> = cap.elements.iter().take(max_elements as usize).cloned().collect();
    let truncated = total.saturating_sub(visible.len());

    let mut summary_lines = vec![format!(
        "capture mode={} {}x{}{}{}",
        cap.mode,
        cap.width,
        cap.height,
        if cap.app.is_empty() {
            String::new()
        } else {
            format!(" app={}", cap.app)
        },
        if cap.window_title.is_empty() {
            String::new()
        } else {
            format!(" window={:?}", cap.window_title)
        }
    ), format!("{total} interactable element(s):")];
    summary_lines.extend(format_elements(&visible, 40));

    if let Some(b64) = cap.png_b64.as_ref().filter(|_| cap.mode != "ax") {
        let summary = summary_lines.join("\n");
        let mime = if b64.starts_with("/9j/") {
            "image/jpeg"
        } else {
            "image/png"
        };
        let screenshot_path = save_screenshot_png(edgecrab_home, b64).ok();
        let envelope = json!({
            "_multimodal": true,
            "content": [
                { "type": "text", "text": summary.clone() },
                { "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{b64}") } }
            ],
            "text_summary": summary,
            "screenshot_path": screenshot_path.as_ref().map(|p| p.display().to_string()),
            "meta": {
                "mode": cap.mode,
                "width": cap.width,
                "height": cap.height,
                "elements": total,
                "png_bytes": cap.png_bytes_len
            }
        });
        return serde_json::to_string(&envelope).map_err(|e| e.to_string());
    }

    if truncated > 0 {
        summary_lines.push(format!(
            "  (response truncated to {} of {total} elements; raise max_elements or pass app= to narrow)",
            visible.len()
        ));
    }
    let summary = summary_lines.join("\n");
    serde_json::to_string(&json!({
        "mode": cap.mode,
        "width": cap.width,
        "height": cap.height,
        "app": cap.app,
        "window_title": cap.window_title,
        "elements": visible.iter().map(element_to_json).collect::<Vec<_>>(),
        "total_elements": total,
        "truncated_elements": truncated,
        "summary": summary,
    }))
    .map_err(|e| e.to_string())
}

pub fn action_response(res: &ActionResult) -> String {
    let mut payload = json!({ "ok": res.ok, "action": res.action });
    if !res.message.is_empty() {
        payload["message"] = json!(res.message);
    }
    if !res.meta.is_empty() {
        payload["meta"] = json!(res.meta);
    }
    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
}

pub fn parse_multimodal_tool_output(text: &str) -> Option<(String, String)> {
    let trimmed = text.lines().next()?.trim();
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    if value.get("_multimodal") != Some(&json!(true)) {
        return None;
    }
    let summary = value
        .get("text_summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let image_url = value
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|parts| {
            parts.iter().find_map(|p| {
                (p.get("type")? == "image_url")
                    .then(|| p.get("image_url")?.get("url")?.as_str().map(str::to_string))
                    .flatten()
            })
        })?;
    Some((summary, image_url))
}

pub fn max_elements_from_args(args: &serde_json::Value) -> u32 {
    coerce_max_elements(args.get("max_elements"))
}
