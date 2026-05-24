//! Abstract backend interface for computer use (mirrors Hermes `backend.py`).

use std::collections::HashMap;

use async_trait::async_trait;

/// One interactable element on the current screen (SOM index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UIElement {
    pub index: u32,
    pub role: String,
    pub label: String,
    pub bounds: (i32, i32, i32, i32),
    pub app: String,
}

impl UIElement {
    #[allow(dead_code)]
    pub fn center(&self) -> (i32, i32) {
        let (x, y, w, h) = self.bounds;
        (x + w / 2, y + h / 2)
    }
}

/// Result of a screen capture call.
#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub mode: String,
    pub width: u32,
    pub height: u32,
    pub png_b64: Option<String>,
    pub elements: Vec<UIElement>,
    pub app: String,
    pub window_title: String,
    pub png_bytes_len: usize,
}

/// Result of any mutating action.
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub ok: bool,
    pub action: String,
    pub message: String,
    pub meta: HashMap<String, serde_json::Value>,
}

/// Lifecycle: `start()` before first use, `stop()` at shutdown.
#[async_trait]
pub trait ComputerUseBackend: Send {
    async fn start(&mut self) -> Result<(), String>;
    #[allow(dead_code)]
    async fn stop(&mut self);
    #[allow(dead_code)]
    fn is_available(&self) -> bool;

    async fn capture(&mut self, mode: &str, app: Option<&str>) -> Result<CaptureResult, String>;

    async fn click(
        &mut self,
        element: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        button: &str,
        click_count: u32,
        modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String>;

    async fn drag(
        &mut self,
        from_element: Option<u32>,
        to_element: Option<u32>,
        from_xy: Option<(i32, i32)>,
        to_xy: Option<(i32, i32)>,
        button: &str,
        modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String>;

    async fn scroll(
        &mut self,
        direction: &str,
        amount: i32,
        element: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String>;

    async fn type_text(&mut self, text: &str) -> Result<ActionResult, String>;
    async fn key(&mut self, keys: &str) -> Result<ActionResult, String>;
    async fn list_apps(&mut self) -> Result<Vec<HashMap<String, serde_json::Value>>, String>;
    async fn focus_app(&mut self, app: &str, raise_window: bool) -> Result<ActionResult, String>;
    async fn set_value(&mut self, value: &str, element: Option<u32>) -> Result<ActionResult, String>;

    async fn wait(&mut self, seconds: f64) -> ActionResult {
        let clamped = seconds.clamp(0.0, 30.0);
        tokio::time::sleep(std::time::Duration::from_secs_f64(clamped)).await;
        ActionResult {
            ok: true,
            action: "wait".into(),
            message: format!("waited {clamped:.2}s"),
            meta: HashMap::new(),
        }
    }
}
