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
    /// AX actions advertised by this element, e.g. `["AXPress"]` or
    /// `["AXPick", "AXCancel"]`. Empty when the AX tree didn't expose any.
    ///
    /// **Why we surface this:** cua-driver's `click` tool defaults to
    /// `AXPress`. Elements whose only supported actions are `AXPick`,
    /// `AXOpen`, or `AXShowMenu` will return `-25206 kAXErrorActionUnsupported`.
    /// Showing the actions lets the model pick the right `action` parameter
    /// instead of crashing into AXPress unconditionally.
    pub actions: Vec<String>,
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

    /// Like `capture`, with an optional `query` to scope the AX tree walk.
    ///
    /// `query` is a case-insensitive substring; cua-driver returns only
    /// matching tree lines plus their ancestor chain, while element_index
    /// values stay valid against the full cached tree (per cua-driver doc).
    /// Default implementation ignores `query` for backward compatibility.
    async fn capture_with_query(
        &mut self,
        mode: &str,
        app: Option<&str>,
        _query: Option<&str>,
    ) -> Result<CaptureResult, String> {
        self.capture(mode, app).await
    }

    /// `ax_action` (last arg): cua-driver AX `action` for element-indexed clicks —
    /// `"press"|"show_menu"|"pick"|"confirm"|"cancel"|"open"`. `None` ⇒ driver
    /// default (press). Ignored when clicking by coordinates.
    ///
    /// 8 parameters are intentional: the underlying cua-driver `click` tool has
    /// the same surface (element|xy + button + count + modifiers + action), and
    /// mirroring the shape keeps backend ↔ MCP mapping 1:1.
    #[allow(clippy::too_many_arguments)]
    async fn click(
        &mut self,
        element: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        button: &str,
        click_count: u32,
        modifiers: Option<&[String]>,
        ax_action: Option<&str>,
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

    async fn type_text(&mut self, text: &str, element: Option<u32>) -> Result<ActionResult, String>;
    async fn key(&mut self, keys: &str) -> Result<ActionResult, String>;
    async fn list_apps(&mut self) -> Result<Vec<HashMap<String, serde_json::Value>>, String>;
    async fn focus_app(&mut self, app: &str, raise_window: bool) -> Result<ActionResult, String>;
    async fn set_value(&mut self, value: &str, element: Option<u32>) -> Result<ActionResult, String>;

    /// Launch a macOS app in the background — does not steal focus or raise window.
    ///
    /// `target` is either a bundle ID (preferred, e.g. `"com.apple.Safari"`) or
    /// an app name (e.g. `"Safari"`). For browsers, `urls` MUST contain at least
    /// one URL or NSWorkspace won't create a window (per cua-driver doc); pass
    /// `["about:blank"]` for a blank tab.
    ///
    /// Default impl: returns an `unsupported` error so backends without this
    /// capability (e.g. `NoopBackend` in tests where unused) don't break.
    async fn launch_app(
        &mut self,
        target: &str,
        urls: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        let _ = (target, urls);
        Err("launch_app not supported by this backend".into())
    }

    /// Open a URL in a browser via `launch_app` (cua-driver primary path — omnibox Return does not commit).
    async fn navigate_url(&mut self, url: &str) -> Result<ActionResult, String> {
        let _ = url;
        Err("navigate_url not supported by this backend".into())
    }

    /// Open `url` in a browser; `app` / `bundle_id` come from the tool call (preferred over stale state).
    async fn open_browser_url(
        &mut self,
        app: Option<&str>,
        bundle_id: Option<&str>,
        url: &str,
        via_action: &str,
    ) -> Result<ActionResult, String> {
        let _ = (app, bundle_id, url, via_action);
        Err("open_browser_url not supported by this backend".into())
    }

    /// Remember a URL typed into a browser so the next `Return` can `launch_app` (omnibox Return is a no-op).
    fn set_pending_browser_url(&mut self, _url: &str) {}

    /// Take the pending URL after `Return`, if any.
    fn take_pending_browser_url(&mut self) -> Option<String> {
        None
    }

    /// App name last targeted by `focus_app` or `capture(app=...)`.
    fn targeted_app(&self) -> Option<&str> {
        None
    }

    /// Resolve `active_pid` / `active_window_id` before a mutating action.
    ///
    /// When `app` is passed on `key`/`type`/`click`/…, or when a prior
    /// `focus_app`/`capture(app=…)` set an intent via `last_app`, the cua
    /// backend will look up the window without requiring a prior `capture()`.
    /// Default: no-op (noop backend tests).
    async fn prepare_action_target(&mut self, app: Option<&str>) -> Result<(), String> {
        let _ = app;
        Ok(())
    }

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
