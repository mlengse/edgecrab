//! OpenAI function schema for `computer_use` (mirrors Hermes `schema.py`).

use edgecrab_types::ToolSchema;
use serde_json::json;

pub const DEFAULT_MAX_ELEMENTS: u32 = 100;
pub const MAX_ALLOWED_MAX_ELEMENTS: u32 = 1000;

pub fn computer_use_schema() -> ToolSchema {
    ToolSchema {
        name: "computer_use".into(),
        description: "Drive the macOS desktop in the background — screenshots, mouse, \
                       keyboard, scroll, drag — without stealing the user's cursor, \
                       keyboard focus, or Space. Preferred workflow: call with \
                       action='capture' (mode='som' gives numbered element overlays), \
                       then click by `element` index for reliability. Pixel coordinates \
                       are supported for models trained on them. Works on any window — \
                       hidden, minimized, on another Space, or behind another app. \
                       macOS only; requires cua-driver to be installed."
            .into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "capture", "click", "double_click", "right_click", "middle_click",
                        "drag", "scroll", "type", "key", "set_value", "wait",
                        "list_apps", "focus_app"
                    ],
                    "description": "Which action to perform."
                },
                "mode": {
                    "type": "string",
                    "enum": ["som", "vision", "ax"],
                    "description": "Capture mode. `som` (default) is best for vision models."
                },
                "app": { "type": "string", "description": "Optional app name or bundle ID." },
                "max_elements": {
                    "type": "integer",
                    "default": DEFAULT_MAX_ELEMENTS,
                    "minimum": 1,
                    "maximum": MAX_ALLOWED_MAX_ELEMENTS,
                    "description": "Cap on AX elements returned by capture."
                },
                "element": { "type": "integer", "description": "1-based SOM index from capture." },
                "coordinate": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "minItems": 2,
                    "maxItems": 2
                },
                "button": { "type": "string", "enum": ["left", "right", "middle"] },
                "modifiers": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["cmd", "shift", "option", "alt", "ctrl", "fn"] }
                },
                "from_element": { "type": "integer" },
                "to_element": { "type": "integer" },
                "from_coordinate": { "type": "array", "items": { "type": "integer" }, "minItems": 2, "maxItems": 2 },
                "to_coordinate": { "type": "array", "items": { "type": "integer" }, "minItems": 2, "maxItems": 2 },
                "direction": { "type": "string", "enum": ["up", "down", "left", "right"] },
                "amount": { "type": "integer", "description": "Scroll wheel ticks. Default 3." },
                "value": { "type": "string", "description": "For set_value." },
                "text": { "type": "string", "description": "Text to type." },
                "keys": { "type": "string", "description": "Key combo, e.g. cmd+s." },
                "seconds": { "type": "number", "description": "Seconds to wait. Max 30." },
                "raise_window": { "type": "boolean", "description": "focus_app only; default false." },
                "capture_after": { "type": "boolean", "description": "Follow-up capture after action." }
            },
            "required": ["action"]
        }),
        strict: None,
    }
}

pub fn coerce_max_elements(value: Option<&serde_json::Value>) -> u32 {
    let Some(v) = value else {
        return DEFAULT_MAX_ELEMENTS;
    };
    let Ok(n) = v.as_i64().ok_or(()) else {
        return DEFAULT_MAX_ELEMENTS;
    };
    if n < 1 {
        return DEFAULT_MAX_ELEMENTS;
    }
    if n > i64::from(MAX_ALLOWED_MAX_ELEMENTS) {
        return MAX_ALLOWED_MAX_ELEMENTS;
    }
    n as u32
}
