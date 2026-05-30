//! Test/CI stub backend (mirrors Hermes `_NoopBackend`).

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::json;

use super::backend::{ActionResult, CaptureResult, ComputerUseBackend};

#[derive(Debug, Default)]
pub struct NoopBackend {
    pub calls: Vec<(String, serde_json::Value)>,
    started: bool,
    pending_url: Option<String>,
}

impl NoopBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn record(&mut self, action: &str, args: serde_json::Value) {
        self.calls.push((action.to_string(), args));
    }
}

#[async_trait]
impl ComputerUseBackend for NoopBackend {
    async fn start(&mut self) -> Result<(), String> {
        self.started = true;
        Ok(())
    }

    async fn stop(&mut self) {
        self.started = false;
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn capture(&mut self, mode: &str, app: Option<&str>) -> Result<CaptureResult, String> {
        self.record(
            "capture",
            serde_json::json!({ "mode": mode, "app": app }),
        );
        Ok(CaptureResult {
            mode: mode.to_string(),
            width: 1024,
            height: 768,
            png_b64: None,
            elements: Vec::new(),
            app: app.unwrap_or("").to_string(),
            window_title: String::new(),
            png_bytes_len: 0,
        })
    }

    async fn click(
        &mut self,
        element: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        button: &str,
        click_count: u32,
        _modifiers: Option<&[String]>,
        ax_action: Option<&str>,
    ) -> Result<ActionResult, String> {
        self.record(
            "click",
            json!({
                "element": element, "x": x, "y": y,
                "button": button, "click_count": click_count,
                "ax_action": ax_action,
            }),
        );
        Ok(ActionResult {
            ok: true,
            action: "click".into(),
            message: String::new(),
            meta: HashMap::new(),
        })
    }

    async fn drag(
        &mut self,
        from_element: Option<u32>,
        to_element: Option<u32>,
        _from_xy: Option<(i32, i32)>,
        _to_xy: Option<(i32, i32)>,
        _button: &str,
        _modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        self.record("drag", json!({ "from_element": from_element, "to_element": to_element }));
        Ok(ActionResult {
            ok: true,
            action: "drag".into(),
            message: String::new(),
            meta: HashMap::new(),
        })
    }

    async fn scroll(
        &mut self,
        direction: &str,
        amount: i32,
        _element: Option<u32>,
        _x: Option<i32>,
        _y: Option<i32>,
        _modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        self.record("scroll", json!({ "direction": direction, "amount": amount }));
        Ok(ActionResult {
            ok: true,
            action: "scroll".into(),
            message: String::new(),
            meta: HashMap::new(),
        })
    }

    async fn type_text(&mut self, text: &str, element: Option<u32>) -> Result<ActionResult, String> {
        self.record("type", serde_json::json!({ "text": text, "element": element }));
        Ok(ActionResult {
            ok: true,
            action: "type".into(),
            message: String::new(),
            meta: HashMap::new(),
        })
    }

    async fn key(&mut self, keys: &str) -> Result<ActionResult, String> {
        self.record("key", serde_json::json!({ "keys": keys }));
        Ok(ActionResult {
            ok: true,
            action: "key".into(),
            message: String::new(),
            meta: HashMap::new(),
        })
    }

    async fn list_apps(&mut self) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        self.record("list_apps", serde_json::json!({}));
        Ok(Vec::new())
    }

    async fn focus_app(&mut self, app: &str, raise_window: bool) -> Result<ActionResult, String> {
        self.record(
            "focus_app",
            serde_json::json!({ "app": app, "raise_window": raise_window }),
        );
        Ok(ActionResult {
            ok: true,
            action: "focus_app".into(),
            message: String::new(),
            meta: HashMap::new(),
        })
    }

    async fn set_value(&mut self, value: &str, element: Option<u32>) -> Result<ActionResult, String> {
        self.record(
            "set_value",
            serde_json::json!({ "value": value, "element": element }),
        );
        Ok(ActionResult {
            ok: true,
            action: "set_value".into(),
            message: String::new(),
            meta: HashMap::new(),
        })
    }

    fn set_pending_browser_url(&mut self, url: &str) {
        self.pending_url = Some(url.to_string());
    }

    fn take_pending_browser_url(&mut self) -> Option<String> {
        self.pending_url.take()
    }

    async fn navigate_url(&mut self, url: &str) -> Result<ActionResult, String> {
        self.open_browser_url(None, None, url, "navigate").await
    }

    async fn open_browser_url(
        &mut self,
        app: Option<&str>,
        bundle_id: Option<&str>,
        url: &str,
        via_action: &str,
    ) -> Result<ActionResult, String> {
        self.record(
            "open_browser_url",
            serde_json::json!({ "app": app, "bundle_id": bundle_id, "url": url, "via": via_action }),
        );
        Ok(ActionResult {
            ok: true,
            action: via_action.into(),
            message: format!("noop open {url}"),
            meta: HashMap::new(),
        })
    }

    async fn launch_app(
        &mut self,
        target: &str,
        urls: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        self.record(
            "launch_app",
            serde_json::json!({ "target": target, "urls": urls }),
        );
        Ok(ActionResult {
            ok: true,
            action: "launch_app".into(),
            message: format!("Launched {target}."),
            meta: HashMap::new(),
        })
    }
}
