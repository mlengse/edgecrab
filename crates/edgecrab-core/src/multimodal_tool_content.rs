//! Tool-result multimodal policy for the conversation → provider boundary.
//!
//! Single place for Hermes-style rules: when to attach `computer_use` screenshots
//! to API requests, when to downgrade to text, and how to recover from rejections.

use std::collections::HashSet;

use edgecrab_tools::config_ref::AppConfigRef;
use edgecrab_tools::vision_models::model_supports_vision;
use edgecrab_tools::{provider_accepts_multimodal_tool_result, should_route_capture_to_aux_vision};
use edgecrab_types::{Content, ContentPart, Message, Role};

/// Provider/model pair that rejected list-shaped tool message content this session.
pub type ProviderModelKey = (String, String);

pub fn provider_model_key(provider: &str, model: &str) -> ProviderModelKey {
    (
        provider.trim().to_ascii_lowercase(),
        model.trim().to_string(),
    )
}

/// Whether `computer_use` tool results should retain inline images in session history.
pub fn should_store_computer_use_images_in_session(
    tool_name: &str,
    provider: &str,
    model: &str,
    config: &AppConfigRef,
    session_downgrades: &HashSet<ProviderModelKey>,
) -> bool {
    if tool_name != "computer_use" {
        return true;
    }
    should_attach_computer_use_screenshot(provider, model, config, session_downgrades)
}

/// Whether to attach the latest `computer_use` disk screenshot when building API messages.
pub fn should_attach_computer_use_screenshot(
    provider: &str,
    model: &str,
    config: &AppConfigRef,
    session_downgrades: &HashSet<ProviderModelKey>,
) -> bool {
    let key = provider_model_key(provider, model);
    if session_downgrades.contains(&key) {
        return false;
    }
    if should_route_capture_to_aux_vision(provider, model, config) {
        return false;
    }
    match provider_accepts_multimodal_tool_result(provider, model) {
        Some(false) => false,
        Some(true) => model_supports_vision(None, provider, model),
        None => false,
    }
}

/// True when the provider rejected tool message ordering / id pairing (not multimodal).
pub fn is_tool_message_order_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("unexpected tool call id")
        || lower.contains("invalid_request_message_order")
        || lower.contains("invalid request message order")
        || lower.contains("code: 3230")
        || lower.contains("code 3230")
}

/// True when a provider error likely means tool messages must not carry image parts.
pub fn is_tool_content_rejection_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    if is_tool_message_order_error(&lower) {
        return false;
    }
    const NEEDLES: &[&str] = &[
        "text is not set",
        "tool message",
        "tool result",
        "tool_use",
        "tool content",
        "invalid type",
        "image_url",
        "multimodal",
        "content must be",
        "unsupported content",
    ];
    NEEDLES.iter().any(|n| lower.contains(n))
        && (lower.contains("tool") || lower.contains("image") || lower.contains("content"))
}

/// Index of the latest `computer_use` tool message with a disk-backed capture (if any).
pub fn last_computer_use_disk_capture_index(messages: &[Message]) -> Option<usize> {
    messages.iter().enumerate().rev().find_map(|(i, m)| {
        if m.role != Role::Tool || m.name.as_deref() != Some("computer_use") {
            return None;
        }
        let Content::Text(text) = m.content.as_ref()? else {
            return None;
        };
        edgecrab_types::multimodal_disk_image_from_content(text).map(|_| i)
    })
}

/// Legacy inline base64 images stored in `Content::Parts` (pre path-only captures).
pub fn legacy_inline_tool_images(content: &Content) -> Option<Vec<edgequake_llm::ImageData>> {
    let Content::Parts(parts) = content else {
        return None;
    };
    let images: Vec<edgequake_llm::ImageData> = parts
        .iter()
        .filter_map(|p| match p {
            ContentPart::ImageUrl { image_url } => {
                let url = &image_url.url;
                url.strip_prefix("data:image/png;base64,")
                    .map(|b64| edgequake_llm::ImageData::new(b64, "image/png"))
                    .or_else(|| {
                        url.strip_prefix("data:image/jpeg;base64,")
                            .map(|b64| edgequake_llm::ImageData::new(b64, "image/jpeg"))
                    })
            }
            _ => None,
        })
        .collect();
    if images.is_empty() {
        None
    } else {
        Some(images)
    }
}

/// Read disk capture and attach as base64 on the API tool message (session stays path-only).
pub fn attach_disk_capture_to_tool_message(
    chat_msg: &mut edgequake_llm::ChatMessage,
    raw_json: &str,
) -> bool {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let Some((path, mime)) = edgecrab_types::multimodal_disk_image_from_content(raw_json) else {
        return false;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return false;
    };
    let b64 = STANDARD.encode(bytes);
    chat_msg.images = Some(vec![edgequake_llm::ImageData::new(b64, mime)]);
    true
}

/// Apply multimodal parts to a provider tool message (inline legacy + optional disk attach).
pub fn enrich_tool_chat_message(
    chat_msg: &mut edgequake_llm::ChatMessage,
    message: &Message,
    attach_computer_use_disk: bool,
    is_latest_disk_capture: bool,
) {
    if let Some(content) = &message.content
        && let Some(images) = legacy_inline_tool_images(content)
    {
        chat_msg.images = Some(images);
    }
    if attach_computer_use_disk
        && chat_msg.images.is_none()
        && is_latest_disk_capture
        && let Some(Content::Text(raw)) = &message.content
    {
        attach_disk_capture_to_tool_message(chat_msg, raw);
    }
}

/// Strip inline tool images from API chat messages; keep text summaries.
///
/// When `record_downgrade` is `Some((provider, model, set))` and a model id is present,
/// records the pair so later turns preemptively omit tool images (Hermes
/// `_no_list_tool_content_models` parity).
///
/// Returns true when at least one tool message was downgraded.
pub fn downgrade_tool_images_in_chat_messages(
    messages: &mut [edgequake_llm::ChatMessage],
    record_downgrade: Option<(&str, &str, &mut HashSet<ProviderModelKey>)>,
) -> bool {
    let mut changed = false;
    for msg in messages.iter_mut() {
        if msg.role != edgequake_llm::ChatRole::Tool {
            continue;
        }
        if msg.images.as_ref().is_some_and(|imgs| !imgs.is_empty()) {
            msg.images = None;
            if msg.content.trim().is_empty() {
                msg.content = "[screenshot omitted — model does not accept tool images]".into();
            } else if !msg.content.contains("image content removed") {
                msg.content
                    .push_str("\n[image content removed — provider rejected tool images]");
            }
            changed = true;
        }
    }
    if changed && let Some((provider, model, downgrades)) = record_downgrade {
        let model = model.trim();
        if !model.is_empty() {
            downgrades.insert(provider_model_key(provider, model));
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tool_content_rejection_phrases() {
        assert!(is_tool_content_rejection_error(
            "Invalid request: text is not set for tool message"
        ));
        assert!(!is_tool_content_rejection_error("rate limit exceeded"));
    }

    #[test]
    fn mistral_tool_call_id_mismatch_is_not_content_rejection() {
        let err = "mistral API error 400: Unexpected tool call id 5wBWzuv53 in tool results type: invalid_request_message_order, code: 3230";
        assert!(is_tool_message_order_error(err));
        assert!(!is_tool_content_rejection_error(err));
    }

    #[test]
    fn copilot_does_not_attach_screenshots() {
        let cfg = AppConfigRef::default();
        let downgrades = HashSet::new();
        assert!(!should_attach_computer_use_screenshot(
            "copilot",
            "gpt-4.1",
            &cfg,
            &downgrades
        ));
    }

    #[test]
    fn anthropic_vision_model_attaches_when_not_downgraded() {
        let cfg = AppConfigRef::default();
        let downgrades = HashSet::new();
        assert!(should_attach_computer_use_screenshot(
            "anthropic",
            "claude-sonnet-4-20250514",
            &cfg,
            &downgrades
        ));
    }

    #[test]
    fn session_downgrade_blocks_attach() {
        let cfg = AppConfigRef::default();
        let mut downgrades = HashSet::new();
        downgrades.insert(("anthropic".into(), "claude-opus-4.6".into()));
        assert!(!should_attach_computer_use_screenshot(
            "anthropic",
            "claude-opus-4.6",
            &cfg,
            &downgrades
        ));
    }

    #[test]
    fn downgrade_strips_tool_images() {
        let mut msgs = vec![edgequake_llm::ChatMessage::tool_result("id", "summary")];
        msgs[0].images = Some(vec![edgequake_llm::ImageData::new("abc", "image/png")]);
        let mut downgrades = HashSet::new();
        assert!(downgrade_tool_images_in_chat_messages(
            &mut msgs,
            Some(("xiaomi", "mimo-v2.5", &mut downgrades))
        ));
        assert!(msgs[0].images.is_none());
        assert!(downgrades.contains(&("xiaomi".into(), "mimo-v2.5".into())));
    }

    #[test]
    fn xiaomi_style_error_classifies_as_rejection() {
        let err = "Error code: 400 - Param Incorrect: text is not set for tool message";
        assert!(is_tool_content_rejection_error(err));
    }

    #[test]
    fn store_policy_matches_attach_for_computer_use() {
        let cfg = AppConfigRef::default();
        let downgrades = HashSet::new();
        assert!(should_store_computer_use_images_in_session(
            "computer_use",
            "anthropic",
            "claude-opus-4.6",
            &cfg,
            &downgrades
        ));
        assert!(!should_store_computer_use_images_in_session(
            "computer_use",
            "copilot",
            "gpt-4.1",
            &cfg,
            &downgrades
        ));
        assert!(should_store_computer_use_images_in_session(
            "file_read",
            "copilot",
            "gpt-4.1",
            &cfg,
            &downgrades
        ));
    }

    #[test]
    fn last_disk_capture_index_picks_latest() {
        let a = Message::tool_result(
            "t1",
            "computer_use",
            r#"{"_multimodal":true,"text_summary":"a"}"#,
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cap.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
        let body = format!(
            r#"{{"_multimodal":true,"_image_path":"{}","_image_mime":"image/png","text_summary":"b","content":[]}}"#,
            path.display()
        );
        let b = Message::tool_result("t2", "computer_use", &body);
        let idx = last_computer_use_disk_capture_index(&[a, b]).expect("index");
        assert_eq!(idx, 1);
    }

    #[test]
    fn attach_disk_capture_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cap.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n").unwrap();
        let raw = format!(
            r#"{{"_multimodal":true,"_image_path":"{}","_image_mime":"image/png","text_summary":"ok","content":[]}}"#,
            path.display()
        );
        let mut chat = edgequake_llm::ChatMessage::tool_result("id", "ok");
        assert!(attach_disk_capture_to_tool_message(&mut chat, &raw));
        assert!(chat.images.as_ref().is_some_and(|i| !i.is_empty()));
    }
}
