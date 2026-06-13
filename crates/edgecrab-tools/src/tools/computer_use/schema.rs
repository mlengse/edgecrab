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
                       keyboard focus, or Space. Preferred workflow: \
                       1) `list_apps` or `focus_app` → 2) `capture(mode='som', app='…')` \
                       for numbered element overlays → 3) `click` by `element` index → \
                       4) `type` or `set_value` → 5) `key` → 6) re-capture. Browsers: \
                       `navigate` or `launch_app(urls=[...])` — not cmd+l+Return. macOS + cua-driver."
            .into(),
        parameters: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "capture", "click", "double_click", "right_click", "middle_click",
                        "drag", "scroll", "type", "key", "set_value", "wait",
                        "list_apps", "focus_app", "launch_app", "navigate"
                    ],
                    "description": "Which action to perform. `capture` is read-only. \
                        Destructive actions may require user approval in the TUI — \
                        reply `session` once per action type to avoid repeated prompts."
                },
                "mode": {
                    "type": "string",
                    "enum": ["som", "vision", "ax"],
                    "description": "Capture mode. `som` (default): screenshot + numbered \
                        overlays + AX tree — best for vision models. `vision`: screenshot only. \
                        `ax`: accessibility tree only (text-only models)."
                },
                "app": {
                    "type": "string",
                    "description": "Target app by name (e.g. 'Safari') or bundle ID. \
                        Pass on `capture`, `focus_app`, and any mutating action (`key`, `type`, \
                        `click`, …) so inputs hit the right process — never rely on Spotlight \
                        (cmd+space) to open apps; use `launch_app` instead."
                },
                "bundle_id": {
                    "type": "string",
                    "description": "macOS bundle identifier for `launch_app` — preferred over `name`. \
                        Examples: com.apple.Safari, com.google.Chrome, com.apple.finder."
                },
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "URLs/paths handed to `launch_app`. **REQUIRED for browsers** \
                        (Safari, Chrome, Firefox, Arc, Brave, Edge): without at least one URL no \
                        window is created and subsequent capture/click fail. Use [\"about:blank\"] \
                        for a blank tab."
                },
                "max_elements": {
                    "type": "integer",
                    "default": DEFAULT_MAX_ELEMENTS,
                    "minimum": 1,
                    "maximum": MAX_ALLOWED_MAX_ELEMENTS,
                    "description": "Cap AX elements returned by capture (default 100, max 1000). \
                        Dense UIs may truncate; re-capture with `app=` to narrow."
                },
                "query": {
                    "type": "string",
                    "description": "Optional case-insensitive substring filter for `capture`. \
                        Trims the AX tree to matching lines + ancestors. Element indices stay valid \
                        against the full tree, so a follow-up `click` with `element=N` still works. \
                        Use to keep large windows like Safari Start Page fast (38s → ~2s)."
                },
                "element": {
                    "type": "integer",
                    "description": "1-based SOM index from the last `capture(mode='som')`. \
                        Use for click/scroll/set_value only — **not** for browser URLs (Hermes: \
                        `type` is pid-wide; use `navigate` or `launch_app(urls=[...])` instead)."
                },
                "url": {
                    "type": "string",
                    "description": "For `navigate`: URL or domain (e.g. https://x.com or x.com). \
                        Opens via launch_app in the targeted browser — reliable background navigation."
                },
                "coordinate": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "minItems": 2,
                    "maxItems": 2,
                    "description": "Pixel [x, y] in logical screen space. Last resort if no element index."
                },
                "button": { "type": "string", "enum": ["left", "right", "middle"] },
                "ax_action": {
                    "type": "string",
                    "enum": ["press", "show_menu", "pick", "confirm", "cancel", "open"],
                    "description": "Element-indexed click only. The AX action to invoke: \
                        `press` (default, fires AXPress — most buttons/links), `pick` (AXPick — \
                        menu items, menu-bar items), `show_menu` (AXShowMenu — context menu), \
                        `confirm` (Return on default button), `cancel` (Escape on dismiss), \
                        `open` (AXOpen — file/folder/URL anchors). If a click returns \
                        `AX action AXPress failed with code -25206`, the element's only supported \
                        actions are in its `actions=[...]` list — pick a matching `ax_action`."
                },
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
                "value": {
                    "type": "string",
                    "description": "For `set_value`: set field/dropdown value directly (reliable for Unicode)."
                },
                "text": {
                    "type": "string",
                    "description": "For `type`: text to insert (pid-wide, Hermes parity). Accents/Unicode use clipboard paste. \
                        For browser URLs use `navigate` or `launch_app(urls=[...])` — not cmd+l+Return."
                },
                "keys": {
                    "type": "string",
                    "description": "Key combo for `key`, e.g. Return, cmd+l, cmd+v."
                },
                "seconds": { "type": "number", "description": "Seconds to wait (max 30)." },
                "raise_window": {
                    "type": "boolean",
                    "description": "focus_app only. Default false (background mode — do not steal focus)."
                },
                "capture_after": {
                    "type": "boolean",
                    "description": "When true, run a follow-up `capture(mode='som')` on the same app \
                        and return action outcome + fresh screenshot in one tool result."
                }
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
