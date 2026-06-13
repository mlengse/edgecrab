//! Shared helpers for `{"_multimodal": true, ...}` tool result envelopes.

use serde_json::Value;

/// First line of tool output (multimodal JSON is single-line).
pub fn first_json_line(content: &str) -> &str {
    content.lines().next().unwrap_or(content).trim()
}

pub fn parse_multimodal_value(content: &str) -> Option<Value> {
    let value: Value = serde_json::from_str(first_json_line(content)).ok()?;
    if value.get("_multimodal") == Some(&Value::Bool(true)) {
        Some(value)
    } else {
        None
    }
}

pub fn is_multimodal_tool_json(content: &str) -> bool {
    parse_multimodal_value(content).is_some()
}

pub fn multimodal_text_summary(content: &str) -> Option<String> {
    let value = parse_multimodal_value(content)?;
    value
        .get("text_summary")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

pub fn multimodal_has_image(content: &str) -> bool {
    let Some(value) = parse_multimodal_value(content) else {
        return false;
    };
    multimodal_value_has_inline_image(&value)
}

pub fn multimodal_value_has_inline_image(value: &Value) -> bool {
    value
        .get("content")
        .and_then(|c| c.as_array())
        .is_some_and(|parts| {
            parts
                .iter()
                .any(|p| p.get("type").and_then(|t| t.as_str()) == Some("image_url"))
        })
}

/// Disk-backed capture image (no inline base64 in session JSON).
pub fn multimodal_disk_image(value: &Value) -> Option<(String, String)> {
    let path = value
        .get("_image_path")
        .or_else(|| value.get("screenshot_path"))
        .and_then(|v| v.as_str())
        .filter(|p| !p.is_empty())?;
    let mime = value
        .get("_image_mime")
        .and_then(|v| v.as_str())
        .unwrap_or("image/png")
        .to_string();
    Some((path.to_string(), mime))
}

pub fn multimodal_disk_image_from_content(content: &str) -> Option<(String, String)> {
    let value = parse_multimodal_value(content)?;
    if multimodal_value_has_inline_image(&value) {
        return None;
    }
    multimodal_disk_image(&value)
}

/// True when tool JSON references any screenshot (inline or disk path).
pub fn capture_has_image_reference(content: &str) -> bool {
    multimodal_has_image(content) || multimodal_disk_image_from_content(content).is_some()
}

/// Strip inline `image_url` parts from a tool output before persisting to session history.
///
/// Path-only `computer_use` envelopes are returned unchanged (images attach at API boundary only).
pub fn strip_inline_images_from_tool_output(tool_name: &str, raw: &str) -> String {
    if tool_name != "computer_use" {
        return raw.to_string();
    }
    let Some(mut value) = parse_multimodal_value(raw) else {
        return raw.to_string();
    };
    if multimodal_disk_image(&value).is_some() && !multimodal_value_has_inline_image(&value) {
        return raw.to_string();
    }
    if let Some(arr) = value.get_mut("content").and_then(|c| c.as_array_mut()) {
        arr.retain(|p| p.get("type").and_then(|t| t.as_str()) != Some("image_url"));
    }
    serde_json::to_string(&value)
        .unwrap_or_else(|_| multimodal_text_summary(raw).unwrap_or_else(|| raw.to_string()))
}

/// Flat token estimate per image block (matches Hermes model_metadata ~1500).
pub const MULTIMODAL_IMAGE_TOKEN_ESTIMATE: usize = 1500;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_multimodal_envelope() {
        let json = r#"{"_multimodal":true,"text_summary":"capture ok","content":[]}"#;
        assert!(is_multimodal_tool_json(json));
        assert_eq!(multimodal_text_summary(json).as_deref(), Some("capture ok"));
    }

    #[test]
    fn disk_image_without_inline_b64() {
        let json = r#"{"_multimodal":true,"_image_path":"/tmp/x.png","_image_mime":"image/png","text_summary":"ok","content":[{"type":"text","text":"ok"}]}"#;
        assert!(!multimodal_has_image(json));
        assert_eq!(
            multimodal_disk_image_from_content(json).map(|(p, _)| p),
            Some("/tmp/x.png".to_string())
        );
        assert!(capture_has_image_reference(json));
    }

    #[test]
    fn strip_inline_removes_image_url_from_envelope() {
        let json = r#"{"_multimodal":true,"text_summary":"cap","content":[{"type":"text","text":"cap"},{"type":"image_url","image_url":{"url":"data:image/png;base64,abc"}}]}"#;
        let stripped = strip_inline_images_from_tool_output("computer_use", json);
        assert!(!stripped.contains("image_url"));
        assert!(stripped.contains("cap"));
    }
}
