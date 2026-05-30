//! Cua-driver backend (macOS only) — port of Hermes `cua_backend.py`.

use std::collections::HashMap;

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use regex::Regex;
use serde_json::{Value, json};

use super::backend::{ActionResult, CaptureResult, ComputerUseBackend, UIElement};
use super::mcp::{CuaMcpSession, McpToolResult};
use super::permissions::{cua_driver_binary_available, install_hint, is_macos};

static WINDOW_LINE_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
static ELEMENT_LINE_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

fn window_line_re() -> &'static Regex {
    WINDOW_LINE_RE.get_or_init(|| {
        Regex::new(r#"^-\s+(.+?)\s+\(pid\s+(\d+)\)\s+.*\[window_id:\s+(\d+)\]"#).expect("regex")
    })
}

fn element_line_re() -> &'static Regex {
    ELEMENT_LINE_RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^\s*(?:-\s+)?\[(\d+)\]\s+(\w+)(?:\s+"([^"]*)"|(?:\s+\(\d+\))?\s+id=([^\s\[\]]*))?"#,
        )
        .expect("regex")
    })
}

/// Parses `actions=[AXPress, AXShowMenu]` trailing fragments from an AX tree line.
/// Returns the list of action names found on `line`, or `None` if absent.
fn parse_actions_for_line(line: &str) -> Option<Vec<String>> {
    static ACTIONS_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = ACTIONS_RE.get_or_init(|| Regex::new(r"actions=\[([^\]]+)\]").expect("regex"));
    let cap = re.captures(line)?;
    let inner = cap.get(1)?.as_str();
    let actions: Vec<String> = inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub struct CuaDriverBackend {
    cmd: String,
    session: Option<CuaMcpSession>,
    active_pid: Option<i32>,
    active_window_id: Option<i32>,
    last_app: Option<String>,
    /// Last element clicked — scoped to `element_scope_*` (invalidated on app/window change).
    last_element: Option<u32>,
    /// SOM indices are only valid for this window until the next capture on another target.
    element_scope_pid: Option<i32>,
    element_scope_window_id: Option<i32>,
    /// Highest element index from the last `som`/`ax` capture on that window.
    element_scope_max_index: Option<u32>,
    /// Set after `cmd+l` + URL `type` — `Return` maps to `launch_app` (omnibox Return is a no-op).
    pending_omnibox_url: Option<String>,
}

impl CuaDriverBackend {
    pub fn new(cmd: impl Into<String>) -> Self {
        Self {
            cmd: cmd.into(),
            session: None,
            active_pid: None,
            active_window_id: None,
            last_app: None,
            last_element: None,
            element_scope_pid: None,
            element_scope_window_id: None,
            element_scope_max_index: None,
            pending_omnibox_url: None,
        }
    }
}

#[async_trait]
impl ComputerUseBackend for CuaDriverBackend {
    async fn start(&mut self) -> Result<(), String> {
        if self.session.is_some() {
            return Ok(());
        }
        if !cua_driver_binary_available(&self.cmd) {
            return Err(install_hint().to_string());
        }
        self.session = Some(
            CuaMcpSession::spawn(&self.cmd, &["mcp"])
                .await
                .map_err(|e| format!("cua-driver MCP start failed: {e}"))?,
        );
        Ok(())
    }

    async fn stop(&mut self) {
        self.session = None;
    }

    fn is_available(&self) -> bool {
        is_macos() && cua_driver_binary_available(&self.cmd)
    }

    async fn capture(&mut self, mode: &str, app: Option<&str>) -> Result<CaptureResult, String> {
        self.capture_with_query(mode, app, None).await
    }

    async fn capture_with_query(
        &mut self,
        mode: &str,
        app: Option<&str>,
        query: Option<&str>,
    ) -> Result<CaptureResult, String> {
        let mut windows = self.fetch_windows().await?;
        if windows.is_empty() {
            return Ok(empty_capture(mode));
        }

        if let Some(filter) = app {
            if let Some(w) = pick_window_for_app(&windows, filter) {
                windows = vec![w];
            } else {
                self.last_app = Some(filter.to_string());
                return Ok(CaptureResult {
                    mode: mode.into(),
                    width: 0,
                    height: 0,
                    png_b64: None,
                    elements: vec![],
                    app: String::new(),
                    window_title: build_focus_app_failure_hint(filter),
                    png_bytes_len: 0,
                });
            }
        }

        // Preserve focus_app / prior capture target — do not reset to frontmost when
        // the agent calls capture() without app= after focusing Safari (Hermes #follow-up).
        let sticky = self.sticky_window_from_list(&windows);
        let target = if app.is_some() {
            windows
                .iter()
                .find(|w| !w.off_screen)
                .or(windows.first())
                .expect("filtered windows non-empty")
                .clone()
        } else if let Some(w) = sticky {
            w
        } else if let Some(last) = self.last_app.as_deref() {
            // Hermes parity: never silently capture the frontmost window when the
            // agent already named a target (failed focus_app, or prior capture with
            // app=). That drift is what caused ExpressVPN captures in the Safari task.
            return Ok(CaptureResult {
                mode: mode.into(),
                width: 0,
                height: 0,
                png_b64: None,
                elements: vec![],
                app: String::new(),
                window_title: build_focus_app_failure_hint(last),
                png_bytes_len: 0,
            });
        } else {
            windows
                .iter()
                .find(|w| !w.off_screen)
                .or(windows.first())
                .expect("windows non-empty")
                .clone()
        };

        self.active_pid = Some(target.pid);
        self.active_window_id = Some(target.window_id);
        if app.is_some() || self.last_app.is_none() {
            self.last_app = Some(target.app_name.clone());
        }

        let mut png_b64 = None;
        let mut elements = Vec::new();
        let mut window_title = target.title.clone();

        let session = self.session.as_mut().ok_or("backend not started")?;
        let (width, height) = if mode == "vision" {
            let sc = session
                .call_tool(
                    "screenshot",
                    json!({
                        "window_id": target.window_id,
                        "format": "jpeg",
                        "quality": 85
                    }),
                )
                .await?;
            png_b64 = sc.images.into_iter().next();
            // Hermes leaves width/height at 0 for vision-only captures; we still
            // fetch dimensions so the TUI never shows misleading `0x0` success.
            let gws = session
                .call_tool(
                    "get_window_state",
                    json!({ "pid": target.pid, "window_id": target.window_id }),
                )
                .await?;
            let parsed = parse_gws_result(&gws);
            self.record_element_scope(target.pid, target.window_id, &[]);
            (parsed.width, parsed.height)
        } else {
            let mut gws_args = json!({ "pid": target.pid, "window_id": target.window_id });
            if let Some(q) = query.filter(|q| !q.is_empty())
                && let Some(obj) = gws_args.as_object_mut()
            {
                obj.insert("query".into(), json!(q));
            }
            let gws = session.call_tool("get_window_state", gws_args).await?;
            let parsed = parse_gws_result(&gws);
            let tree = parsed.tree;
            if !tree.is_empty() && gws.images.is_empty() {
                elements = parse_elements(&tree);
            } else if let Some(img) = gws.images.into_iter().next() {
                png_b64 = Some(img);
                elements = parse_elements(&tree);
            }
            if let Some(cap) = Regex::new(r#"AXWindow\s+"([^"]+)""#)
                .ok()
                .and_then(|re| re.captures(&tree))
            {
                window_title = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
            self.record_element_scope(target.pid, target.window_id, &elements);
            if elements.is_empty() {
                if let Some(expected) = parsed.element_count.filter(|n| *n > 0) {
                    window_title = format!(
                        "{window_title} [AX tree: {expected} elements reported but none parsed — \
                         retry capture or check Accessibility permission]"
                    );
                } else if gws.is_error {
                    let hint = parsed.summary;
                    if !hint.is_empty() {
                        window_title = format!("{window_title} [{hint}]");
                    }
                } else if !parsed.summary.is_empty() && parsed.summary.contains('⚠') {
                    window_title = format!(
                        "{window_title} [{}]",
                        parsed.summary.lines().next().unwrap_or("")
                    );
                }
            }
            (parsed.width, parsed.height)
        };

        let png_bytes_len = png_b64
            .as_ref()
            .and_then(|b64| STANDARD.decode(b64).ok())
            .map(|raw| raw.len())
            .unwrap_or(0);

        Ok(CaptureResult {
            mode: mode.into(),
            width,
            height,
            png_b64,
            elements,
            app: target.app_name,
            window_title,
            png_bytes_len,
        })
    }

    async fn prepare_action_target(&mut self, app: Option<&str>) -> Result<(), String> {
        self.ensure_active_target(app).await
    }

    async fn click(
        &mut self,
        element: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        button: &str,
        click_count: u32,
        modifiers: Option<&[String]>,
        ax_action: Option<&str>,
    ) -> Result<ActionResult, String> {
        self.ensure_active_target(None).await?;
        let pid = self
            .active_pid
            .ok_or("No active window — call capture() first.")?;
        let tool = match (button, click_count) {
            ("right", _) => "right_click",
            (_, 2) => "double_click",
            _ => "click",
        };
        let mut args = json!({ "pid": pid });
        if let Some(el) = element {
            self.validate_element_index(el)?;
            let wid = self
                .active_window_id
                .ok_or("No active window_id for element_index click.")?;
            args["element_index"] = json!(el);
            args["window_id"] = json!(wid);
            // AX action only meaningful in element-indexed mode (cua-driver doc).
            if let Some(act) = ax_action.filter(|s| !s.is_empty()) {
                args["action"] = json!(act);
            }
        } else if let (Some(x), Some(y)) = (x, y) {
            args["x"] = json!(x);
            args["y"] = json!(y);
        } else {
            return Err(format!("{tool} requires element= or x/y."));
        }
        if let Some(mods) = modifiers {
            args["modifier"] = json!(mods);
        }
        let result = self.action(tool, args).await?;
        if element.is_some() {
            self.last_element = element;
        }
        Ok(result)
    }

    async fn drag(
        &mut self,
        from_element: Option<u32>,
        to_element: Option<u32>,
        from_xy: Option<(i32, i32)>,
        to_xy: Option<(i32, i32)>,
        _button: &str,
        _modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        self.ensure_active_target(None).await?;
        let pid = self
            .active_pid
            .ok_or("No active window — call capture() first.")?;
        let mut args = json!({ "pid": pid });
        if let (Some(f), Some(t)) = (from_element, to_element) {
            self.validate_element_index(f)?;
            self.validate_element_index(t)?;
            let wid = self
                .active_window_id
                .ok_or("No active window_id for element-based drag.")?;
            args["from_element"] = json!(f);
            args["to_element"] = json!(t);
            args["window_id"] = json!(wid);
        } else if let (Some((fx, fy)), Some((tx, ty))) = (from_xy, to_xy) {
            args["from_x"] = json!(fx);
            args["from_y"] = json!(fy);
            args["to_x"] = json!(tx);
            args["to_y"] = json!(ty);
        } else {
            return Err(
                "drag requires from_element/to_element or from_coordinate/to_coordinate.".into(),
            );
        }
        self.action("drag", args).await
    }

    async fn scroll(
        &mut self,
        direction: &str,
        amount: i32,
        element: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        _modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        self.ensure_active_target(None).await?;
        let pid = self
            .active_pid
            .ok_or("No active window — call capture() first.")?;
        let mut args = json!({
            "pid": pid,
            "direction": direction,
            "amount": amount.clamp(1, 50)
        });
        if let Some(el) = element {
            self.validate_element_index(el)?;
            if let Some(wid) = self.active_window_id {
                args["element_index"] = json!(el);
                args["window_id"] = json!(wid);
            }
        } else if let (Some(x), Some(y)) = (x, y) {
            args["x"] = json!(x);
            args["y"] = json!(y);
        }
        self.action("scroll", args).await
    }

    async fn type_text(
        &mut self,
        text: &str,
        element: Option<u32>,
    ) -> Result<ActionResult, String> {
        use super::text_input::{copy_to_macos_clipboard, needs_clipboard_paste};

        self.ensure_active_target(None).await?;

        // Hermes: type_text is always pid-wide — never element_index (schema must not steer models to element= on URLs).
        if element.is_some() {
            tracing::warn!(
                "computer_use type: element= ignored (Hermes pid-wide typing); use set_value for a specific field"
            );
        }

        if super::browsers::should_open_url_via_launch(None, self.last_app.as_deref(), text) {
            return self.open_browser_url(None, None, text, "type").await;
        }

        let pid = self
            .active_pid
            .ok_or("No active window — call capture() first.")?;

        if needs_clipboard_paste(text) {
            copy_to_macos_clipboard(text)?;
            let mut pasted = self.key("cmd+v").await?;
            pasted.action = "type".into();
            pasted.message = format!(
                "pasted {} chars via clipboard (unicode-safe)",
                text.chars().count()
            );
            return Ok(pasted);
        }

        let mut args = json!({ "pid": pid, "text": text });
        attach_window_id(&mut args, self.active_window_id);
        let mut res = self.action("type_text", args).await?;
        res.action = "type".into();
        Ok(res)
    }

    async fn navigate_url(&mut self, url_text: &str) -> Result<ActionResult, String> {
        self.open_browser_url(None, None, url_text, "navigate")
            .await
    }

    async fn open_browser_url(
        &mut self,
        app: Option<&str>,
        bundle_id: Option<&str>,
        url_text: &str,
        via_action: &str,
    ) -> Result<ActionResult, String> {
        use super::browsers::{is_browser_app, normalize_nav_url, resolve_launch_target};

        let app_ctx = app.or(self.last_app.as_deref());
        if let Some(name) = app_ctx
            && !is_browser_app(name)
            && bundle_id.is_none()
        {
            return Err(format!(
                "open_browser_url only works for browsers (Chrome, Safari, …); got '{name}'."
            ));
        }
        let target = resolve_launch_target(app, bundle_id, self.last_app.as_deref());
        let url = normalize_nav_url(url_text);
        if url.is_empty() {
            return Err("Empty URL.".into());
        }
        self.pending_omnibox_url = None;
        self.navigate_via_launch_app(&target, &url, via_action)
            .await
    }

    async fn key(&mut self, keys: &str) -> Result<ActionResult, String> {
        use super::text_input::is_address_bar_focus_combo;

        self.ensure_active_target(None).await?;
        let pid = self
            .active_pid
            .ok_or("No active window — call capture() first.")?;

        if is_address_bar_focus_combo(keys) {
            self.pending_omnibox_url = None;
        }

        let (key, mods) = parse_key_combo(keys);
        let key = key.ok_or_else(|| format!("Could not parse key from '{keys}'."))?;

        if mods.is_empty() && (key == "return" || key == "enter") {
            if let Some(url) = self.pending_omnibox_url.take() {
                let bundle = super::browsers::browser_bundle_id(
                    self.last_app.as_deref().unwrap_or("Google Chrome"),
                )
                .unwrap_or("com.google.Chrome");
                return self
                    .navigate_via_launch_app(
                        bundle,
                        &super::browsers::normalize_nav_url(&url),
                        "key",
                    )
                    .await;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        if mods.is_empty() {
            let mut res = self.press_key_raw(pid, &key).await?;
            res.action = "key".into();
            Ok(res)
        } else {
            let mut args = json!({
                "pid": pid,
                "keys": mods.into_iter().chain([key]).collect::<Vec<_>>()
            });
            attach_window_id(&mut args, self.active_window_id);
            let mut res = self.action("hotkey", args).await?;
            res.action = "key".into();
            Ok(res)
        }
    }

    async fn list_apps(&mut self) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        let out = self.mcp_call("list_apps", json!({})).await?;
        parse_list_apps(&out)
    }

    async fn focus_app(&mut self, app: &str, _raise_window: bool) -> Result<ActionResult, String> {
        let windows = self.fetch_windows().await?;
        if let Some(t) = pick_window_for_app(&windows, app) {
            self.apply_window_target(&t);
            return Ok(ActionResult {
                ok: true,
                action: "focus_app".into(),
                message: format!(
                    "Targeted {} (pid {}, window {}) without raising window.",
                    t.app_name, t.pid, t.window_id
                ),
                meta: HashMap::new(),
            });
        }
        // Remember intent so capture()/key() won't drift to the frontmost app.
        self.last_app = Some(app.to_string());
        Ok(ActionResult {
            ok: false,
            action: "focus_app".into(),
            message: build_focus_app_failure_hint(app),
            meta: HashMap::new(),
        })
    }

    async fn launch_app(
        &mut self,
        target: &str,
        urls: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        // Per cua-driver docs: bundle_id wins over name when both passed; browsers
        // require at least one URL (otherwise NSWorkspace starts the process but
        // never creates a window, and subsequent get_window_state/click fail).
        let mut args = if target.contains('.') {
            // Heuristic: bundle IDs always contain at least one dot.
            json!({ "bundle_id": target })
        } else {
            json!({ "name": target })
        };
        if let (Some(u), Some(obj)) = (urls, args.as_object_mut())
            && !u.is_empty()
        {
            obj.insert("urls".into(), json!(u));
        }
        let result = self.action("launch_app", args).await?;
        if !result.ok {
            return Ok(result);
        }
        // Poll for the new window (browsers need urls[] or no window appears).
        let wait_hint = app_match_filters(target)
            .first()
            .cloned()
            .unwrap_or_else(|| target.to_string());
        self.last_app = Some(wait_hint.clone());
        if let Some(w) = self.wait_for_window(target, 8.0).await {
            self.apply_window_target(&w);
            return Ok(ActionResult {
                ok: true,
                action: "launch_app".into(),
                message: format!(
                    "Launched {} (pid {}, window {}). Ready for capture/key — pass app='{wait_hint}' on actions.",
                    w.app_name, w.pid, w.window_id
                ),
                meta: HashMap::new(),
            });
        }
        Ok(ActionResult {
            ok: true,
            action: "launch_app".into(),
            message: format!(
                "{} Process started but no on-screen window yet — retry focus_app(app='{wait_hint}') \
                 or capture(app='{wait_hint}') in ~1s. For browsers confirm urls=['about:blank'] was passed.",
                if result.message.is_empty() {
                    "Launched.".to_string()
                } else {
                    format!("{}.", result.message.trim_end_matches('.'))
                }
            ),
            meta: HashMap::new(),
        })
    }

    async fn set_value(
        &mut self,
        value: &str,
        element: Option<u32>,
    ) -> Result<ActionResult, String> {
        self.ensure_active_target(None).await?;
        let pid = self
            .active_pid
            .ok_or("No active window — call capture() first.")?;
        let wid = self
            .active_window_id
            .ok_or("No active window — call capture() first.")?;
        let el = element.ok_or("set_value requires element= (element index).")?;
        self.validate_element_index(el)?;
        self.action(
            "set_value",
            json!({ "pid": pid, "window_id": wid, "element_index": el, "value": value }),
        )
        .await
    }

    fn targeted_app(&self) -> Option<&str> {
        self.last_app.as_deref()
    }

    fn set_pending_browser_url(&mut self, url: &str) {
        self.pending_omnibox_url = Some(url.to_string());
    }

    fn take_pending_browser_url(&mut self) -> Option<String> {
        self.pending_omnibox_url.take()
    }
}

impl CuaDriverBackend {
    fn apply_window_target(&mut self, w: &WindowRow) {
        let window_changed =
            self.active_pid != Some(w.pid) || self.active_window_id != Some(w.window_id);
        self.active_pid = Some(w.pid);
        self.active_window_id = Some(w.window_id);
        self.last_app = Some(w.app_name.clone());
        if window_changed {
            self.clear_element_scope();
            self.pending_omnibox_url = None;
        }
    }

    fn clear_element_scope(&mut self) {
        self.last_element = None;
        self.element_scope_pid = None;
        self.element_scope_window_id = None;
        self.element_scope_max_index = None;
    }

    async fn press_key_raw(&mut self, pid: i32, key: &str) -> Result<ActionResult, String> {
        let mut args = json!({ "pid": pid, "key": key });
        attach_window_id(&mut args, self.active_window_id);
        self.action("press_key", args).await
    }

    /// `launch_app({bundle_id, urls})` — cua-driver's reliable browser navigation path.
    async fn navigate_via_launch_app(
        &mut self,
        target: &str,
        url: &str,
        via_action: &str,
    ) -> Result<ActionResult, String> {
        let bundle = target;
        let result = self
            .action("launch_app", json!({ "bundle_id": bundle, "urls": [url] }))
            .await?;
        if !result.ok {
            let mut res = result;
            res.action = via_action.into();
            return Ok(res);
        }
        self.last_app = Some(
            app_match_filters(bundle)
                .first()
                .cloned()
                .unwrap_or_else(|| bundle.to_string()),
        );
        // Short poll — full 8s wait is for cold launch; URL open is usually fast.
        if let Some(w) = self.wait_for_window(target, 4.0).await {
            self.apply_window_target(&w);
        }
        Ok(ActionResult {
            ok: true,
            action: via_action.into(),
            message: format!(
                "Opened {url} via launch_app ({target}) [cu-nav-v2]. Active window switched — \
                 Re-capture(app='{}') to verify. Do not use cmd+l+type+Return.",
                self.last_app.as_deref().unwrap_or("Google Chrome")
            ),
            meta: HashMap::new(),
        })
    }

    fn record_element_scope(&mut self, pid: i32, window_id: i32, elements: &[UIElement]) {
        self.element_scope_pid = Some(pid);
        self.element_scope_window_id = Some(window_id);
        self.element_scope_max_index = elements.iter().map(|e| e.index).max();
        // New capture on this window — drop sticky click index from a prior tree.
        self.last_element = None;
    }

    fn validate_element_index(&self, el: u32) -> Result<(), String> {
        validate_element_scope(
            el,
            self.element_scope_pid,
            self.element_scope_window_id,
            self.element_scope_max_index,
            self.active_pid,
            self.active_window_id,
            self.last_app.as_deref(),
        )
    }

    /// Resolve element index for `type` — Hermes parity: pid-wide typing by default.
    ///
    /// When `element` is omitted, cua-driver types into the **focused** field via
    /// real keyboard events (`type_text` without `element_index`). That is required
    /// for browser URL bars after `cmd+l` — AX bulk insert on a stale SOM index
    /// leaves the URL visible but `Return` does not navigate.
    ///
    /// Pass `element=` only when intentionally targeting a specific SOM index
    /// (e.g. Notes search field). Never implied from `last_element`.
    #[allow(dead_code)] // dispatch no longer passes element= on type (Hermes parity); kept for unit tests
    pub(crate) fn resolve_type_element_index(
        &self,
        explicit: Option<u32>,
    ) -> Result<Option<u32>, String> {
        match explicit {
            None => Ok(None),
            Some(e) => {
                self.validate_element_index(e)?;
                Ok(Some(e))
            }
        }
    }

    fn sticky_window_from_list(&self, windows: &[WindowRow]) -> Option<WindowRow> {
        if let (Some(pid), Some(wid)) = (self.active_pid, self.active_window_id)
            && let Some(w) = windows.iter().find(|w| w.pid == pid && w.window_id == wid)
        {
            return Some(w.clone());
        }
        self.last_app
            .as_ref()
            .and_then(|last| pick_window_for_app(windows, last))
    }

    async fn fetch_windows(&mut self) -> Result<Vec<WindowRow>, String> {
        let lw = self
            .mcp_call("list_windows", json!({ "on_screen_only": true }))
            .await?;
        Ok(parse_windows(&lw))
    }

    async fn ensure_active_target(&mut self, app_hint: Option<&str>) -> Result<(), String> {
        let hint_owned = app_hint
            .map(str::to_string)
            .or_else(|| self.last_app.clone());
        if let Some(hint) = hint_owned.as_deref() {
            if let Some(w) = pick_window_for_app(&self.fetch_windows().await?, hint) {
                self.apply_window_target(&w);
                return Ok(());
            }
            if self.active_pid.is_some() {
                // Stale pid from a different app — clear so we don't hit ExpressVPN.
                self.active_pid = None;
                self.active_window_id = None;
            }
            return Err(build_focus_app_failure_hint(hint));
        }
        if self.active_pid.is_some() {
            // Validate pid still exists; clear if the window closed.
            let windows = self.fetch_windows().await?;
            if self.sticky_window_from_list(&windows).is_some() {
                return Ok(());
            }
            self.active_pid = None;
            self.active_window_id = None;
        }
        Err(
            "No active window — call focus_app(app='…') or launch_app first, or pass app= on this action. \
             Do NOT use Spotlight (cmd+space) to open apps; use launch_app with bundle_id + urls for browsers."
                .into(),
        )
    }

    async fn wait_for_window(&mut self, target: &str, max_secs: f64) -> Option<WindowRow> {
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs_f64(max_secs.max(0.5));
        while std::time::Instant::now() < deadline {
            if let Ok(windows) = self.fetch_windows().await
                && let Some(w) = pick_window_for_app(&windows, target)
            {
                return Some(w);
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
        None
    }

    async fn action(
        &mut self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<ActionResult, String> {
        let out = self.mcp_call(name, args).await?;
        let message = match &out.data {
            serde_json::Value::Object(map) => map
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        };
        Ok(ActionResult {
            ok: !out.is_error,
            action: name.into(),
            message,
            meta: HashMap::new(),
        })
    }

    /// True for transient cua-driver failures that a fresh MCP session recovers
    /// from. cua-driver re-attaches its background daemon on the next call, so a
    /// single reconnect+retry turns these one-shot blips into successes instead
    /// of surfacing them to the model as hard failures.
    fn is_transient_daemon_error(err: &str) -> bool {
        let e = err.to_ascii_lowercase();
        e.contains("daemon transport")
            || e.contains("daemon closed")
            || e.contains("closed connection")
            || e.contains("broken pipe")
            || e.contains("timed out")
    }

    async fn reconnect(&mut self) -> Result<(), String> {
        self.session = None;
        self.start().await
    }

    /// Call a cua-driver MCP tool with one automatic reconnect+retry on a
    /// transient daemon-transport failure.
    async fn mcp_call(
        &mut self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<McpToolResult, String> {
        if self.session.is_none() {
            self.start().await?;
        }
        let first = {
            let session = self.session.as_mut().ok_or("backend not started")?;
            session.call_tool(name, args.clone()).await
        };
        match first {
            Ok(out) => Ok(out),
            Err(e) if Self::is_transient_daemon_error(&e) => {
                tracing::warn!(
                    "computer_use: transient cua-driver error on '{name}' ({e}); reconnecting + retrying once"
                );
                self.reconnect().await?;
                let session = self.session.as_mut().ok_or("backend not started")?;
                session.call_tool(name, args).await
            }
            Err(e) => Err(e),
        }
    }

    #[allow(dead_code)]
    pub fn last_app(&self) -> Option<&str> {
        self.last_app.as_deref()
    }
}

struct WindowRow {
    app_name: String,
    pid: i32,
    window_id: i32,
    title: String,
    off_screen: bool,
    z_index: i32,
}

/// Parsed `get_window_state` payload (cua-driver 0.2+ puts tree in structuredContent).
struct GwsParsed {
    summary: String,
    tree: String,
    width: u32,
    height: u32,
    element_count: Option<u32>,
}

impl Clone for WindowRow {
    fn clone(&self) -> Self {
        Self {
            app_name: self.app_name.clone(),
            pid: self.pid,
            window_id: self.window_id,
            title: self.title.clone(),
            off_screen: self.off_screen,
            z_index: self.z_index,
        }
    }
}

fn parse_windows(lw: &super::mcp::McpToolResult) -> Vec<WindowRow> {
    if let Some(sc) = lw.structured.as_ref().and_then(|v| v.get("windows"))
        && let Some(arr) = sc.as_array()
    {
        let mut windows: Vec<WindowRow> = arr
            .iter()
            .filter_map(|w| {
                Some(WindowRow {
                    app_name: w.get("app_name")?.as_str()?.to_string(),
                    pid: w.get("pid")?.as_i64()? as i32,
                    window_id: w.get("window_id")?.as_i64()? as i32,
                    title: w
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    off_screen: !w
                        .get("is_on_screen")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    z_index: w.get("z_index").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                })
            })
            .collect();
        // Lowest z_index = frontmost on macOS (matches Hermes cua_backend.py).
        windows.sort_by_key(|w| w.z_index);
        return windows;
    }
    let text = lw.data.as_str().unwrap_or("");
    window_line_re()
        .captures_iter(text)
        .map(|cap| WindowRow {
            app_name: cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default(),
            pid: cap
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0),
            window_id: cap
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0),
            title: String::new(),
            off_screen: cap
                .get(0)
                .map(|m| m.as_str().contains("[off-screen]"))
                .unwrap_or(false),
            z_index: 0,
        })
        .collect()
}

fn split_tree(full: &str) -> (String, String) {
    match full.split_once('\n') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (full.to_string(), String::new()),
    }
}

fn u32_field(obj: &Value, key: &str) -> u32 {
    obj.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as u32
}

fn merge_gws_fields(into: &mut GwsParsed, obj: &Value) {
    if into.tree.is_empty()
        && let Some(tm) = obj.get("tree_markdown").and_then(|v| v.as_str())
    {
        into.tree = tm.to_string();
    }
    if into.width == 0 {
        into.width = u32_field(obj, "screenshot_width");
    }
    if into.height == 0 {
        into.height = u32_field(obj, "screenshot_height");
    }
    if into.element_count.is_none() {
        into.element_count = obj
            .get("element_count")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);
    }
}

/// Merge cua-driver 0.2+ structuredContent with legacy text/JSON `data` fields.
fn parse_gws_result(gws: &super::mcp::McpToolResult) -> GwsParsed {
    let mut parsed = GwsParsed {
        summary: String::new(),
        tree: String::new(),
        width: 0,
        height: 0,
        element_count: None,
    };

    if let Some(sc) = &gws.structured {
        merge_gws_fields(&mut parsed, sc);
    }

    match &gws.data {
        Value::Object(map) => merge_gws_fields(&mut parsed, &Value::Object(map.clone())),
        Value::String(text) => {
            let (summary, tree) = split_tree(text);
            if parsed.tree.is_empty() {
                parsed.tree = tree;
                if parsed.summary.is_empty() {
                    parsed.summary = summary;
                }
            } else if parsed.summary.is_empty() {
                parsed.summary = summary;
            }
        }
        _ => {}
    }

    parsed
}

fn attach_window_id(args: &mut Value, window_id: Option<i32>) {
    if let Some(wid) = window_id
        && let Some(obj) = args.as_object_mut()
    {
        obj.insert("window_id".into(), json!(wid));
    }
}

fn parse_elements(tree: &str) -> Vec<UIElement> {
    // Iterate per line so we can extract trailing `actions=[...]` from the
    // same line the regex matched against. (The element regex stops before the
    // tail, so capturing it from regex group 0 won't include the actions list.)
    tree.lines()
        .filter_map(|line| {
            let cap = element_line_re().captures(line)?;
            Some(UIElement {
                index: cap
                    .get(1)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(0),
                role: cap
                    .get(2)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
                label: cap
                    .get(3)
                    .or_else(|| cap.get(4))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
                bounds: (0, 0, 0, 0),
                app: String::new(),
                actions: parse_actions_for_line(line).unwrap_or_default(),
            })
        })
        .collect()
}

fn parse_key_combo(keys: &str) -> (Option<String>, Vec<String>) {
    const MODS: &[&str] = &[
        "cmd", "command", "shift", "option", "alt", "ctrl", "control", "fn",
    ];
    let mut modifiers = Vec::new();
    let mut key = None;
    for part in keys.split(['+', '-']) {
        let p = part.trim().to_ascii_lowercase();
        if p.is_empty() {
            continue;
        }
        let norm = match p.as_str() {
            "command" => "cmd".to_string(),
            "alt" => "option".to_string(),
            "control" => "ctrl".to_string(),
            other => other.to_string(),
        };
        if MODS.contains(&norm.as_str()) {
            modifiers.push(norm);
        } else {
            key = Some(normalize_cua_key_name(&norm));
        }
    }
    (key, modifiers)
}

/// cua-driver expects lowercase key names (`return`, not `Return`) — Hermes lowercases all parts.
fn normalize_cua_key_name(key: &str) -> String {
    match key {
        "return" | "enter" => "return".into(),
        "esc" | "escape" => "escape".into(),
        other => other.to_string(),
    }
}

/// Validate an SOM element index against the last capture scope (unit-testable).
pub(crate) fn validate_element_scope(
    el: u32,
    scope_pid: Option<i32>,
    scope_wid: Option<i32>,
    scope_max: Option<u32>,
    active_pid: Option<i32>,
    active_wid: Option<i32>,
    last_app: Option<&str>,
) -> Result<(), String> {
    let (scope_pid, scope_wid, max_idx) = match (scope_pid, scope_wid, scope_max) {
        (Some(p), Some(w), Some(m)) => (p, w, m),
        _ => {
            return Err(
                "element= requires a prior capture(mode='som') on this window — indices are \
                 not valid after focus_app alone. Re-capture with app=, or omit element= \
                 (Hermes types pid-wide)."
                    .into(),
            );
        }
    };
    let (active_pid, active_wid) = match (active_pid, active_wid) {
        (Some(p), Some(w)) => (p, w),
        _ => return Err("No active window.".into()),
    };
    if scope_pid != active_pid || scope_wid != active_wid {
        return Err(format!(
            "element #{el} belongs to pid {scope_pid} window {scope_wid}, but the active \
             target is pid {active_pid} window {active_wid}. SOM indices do not transfer \
             across apps — re-capture(mode='som', app='{}') on the new target, or omit \
             element= for pid-wide typing.",
            last_app.unwrap_or("?")
        ));
    }
    if el > max_idx {
        return Err(format!(
            "element #{el} is out of range for the last capture on this window (max #{max_idx}). \
             The AX cache may have changed — re-capture(mode='som', app='{}') and use a fresh index.",
            last_app.unwrap_or("?")
        ));
    }
    Ok(())
}

/// Name filters for matching a user/bundle target against `list_windows` app_name.
pub(crate) fn app_match_filters(target: &str) -> Vec<String> {
    if target.contains('.') {
        let aliases: &[&str] = match target {
            "com.apple.Safari" => &["Safari"],
            "com.google.Chrome" => &["Google Chrome", "Chrome"],
            "org.mozilla.firefox" => &["Firefox"],
            "company.thebrowser.Browser" => &["Arc"],
            "com.brave.Browser" => &["Brave Browser", "Brave"],
            "com.microsoft.edgemac" => &["Microsoft Edge", "Edge"],
            "com.apple.finder" => &["Finder"],
            "com.apple.Terminal" => &["Terminal"],
            "com.apple.mail" => &["Mail"],
            "com.apple.Notes" => &["Notes"],
            _ => &[],
        };
        if aliases.is_empty() {
            vec![target.to_string()]
        } else {
            aliases.iter().map(|s| (*s).to_string()).collect()
        }
    } else {
        vec![target.to_string()]
    }
}

fn window_matches_filter(w: &WindowRow, filter: &str) -> bool {
    let fl = filter.to_ascii_lowercase();
    w.app_name.to_ascii_lowercase().contains(&fl)
}

fn pick_window_for_app(windows: &[WindowRow], app: &str) -> Option<WindowRow> {
    for filter in app_match_filters(app) {
        if let Some(w) = windows
            .iter()
            .filter(|w| !w.off_screen)
            .find(|w| window_matches_filter(w, &filter))
            .or_else(|| windows.iter().find(|w| window_matches_filter(w, &filter)))
        {
            return Some(w.clone());
        }
    }
    None
}

/// Build the actionable error message for a `focus_app` miss.
///
/// `list_apps` only returns apps with an **on-screen window**, so a missing
/// app means either (a) it's not running, (b) it's running but minimized/
/// hidden, or (c) for browsers, it's running headless without a window.
/// The agent needs concrete next steps — point at `launch_app` with the
/// right bundle ID + URL pattern per the cua-driver doc:
/// "BROWSER WINDOW REQUIREMENT: … use `urls=['about:blank']`".
pub(crate) fn build_focus_app_failure_hint(app: &str) -> String {
    let app_lc = app.to_ascii_lowercase();
    let (bundle, browser) = match app_lc.as_str() {
        "safari" => (Some("com.apple.Safari"), true),
        "chrome" | "google chrome" => (Some("com.google.Chrome"), true),
        "firefox" => (Some("org.mozilla.firefox"), true),
        "arc" => (Some("company.thebrowser.Browser"), true),
        "brave" | "brave browser" => (Some("com.brave.Browser"), true),
        "edge" | "microsoft edge" => (Some("com.microsoft.edgemac"), true),
        "finder" => (Some("com.apple.finder"), false),
        "terminal" => (Some("com.apple.Terminal"), false),
        "notes" => (Some("com.apple.Notes"), false),
        _ => (None, false),
    };
    let recovery = match (bundle, browser) {
        (Some(b), true) => format!(
            "RECOVERY: `launch_app(bundle_id=\"{b}\", urls=[\"about:blank\"])` then retry \
             focus_app. Browsers REQUIRE urls=[…] or no window is created."
        ),
        (Some(b), false) => {
            format!("RECOVERY: `launch_app(bundle_id=\"{b}\")` then retry focus_app.")
        }
        (None, _) => format!(
            "RECOVERY: 1) Run `list_apps` to see exact (localized) names — macOS reports \
             names in the user's language. 2) If the app is not running, try \
             `launch_app(name=\"{app}\")`; for browsers ALSO pass `urls=[\"about:blank\"]`."
        ),
    };
    format!("No on-screen window found for app '{app}'. {recovery}")
}

fn empty_capture(mode: &str) -> CaptureResult {
    CaptureResult {
        mode: mode.into(),
        width: 0,
        height: 0,
        png_b64: None,
        elements: vec![],
        app: String::new(),
        window_title: "No on-screen windows visible. Grant Screen Recording + Accessibility \
                        (/computer open), then retry. If permissions are granted, ensure at \
                        least one app window is visible."
            .into(),
        png_bytes_len: 0,
    }
}

fn parse_list_apps(
    out: &super::mcp::McpToolResult,
) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
    match &out.data {
        serde_json::Value::Array(arr) => Ok(arr
            .iter()
            .filter_map(|v| {
                v.as_object()
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            })
            .collect()),
        serde_json::Value::Object(map) => {
            if let Some(apps) = map.get("apps").and_then(|v| v.as_array()) {
                return Ok(apps
                    .iter()
                    .filter_map(|v| {
                        v.as_object()
                            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    })
                    .collect());
            }
            Ok(Vec::new())
        }
        serde_json::Value::String(text) => {
            static APP_LINE_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
            let re =
                APP_LINE_RE.get_or_init(|| Regex::new(r"(.+?)\s+\(pid\s+(\d+)\)").expect("regex"));
            let mut apps = Vec::new();
            for line in text.lines() {
                if let Some(cap) = re.captures(line) {
                    let mut row = HashMap::new();
                    row.insert(
                        "name".into(),
                        json!(cap.get(1).map(|m| m.as_str().trim()).unwrap_or("")),
                    );
                    if let Some(pid) = cap.get(2).and_then(|m| m.as_str().parse::<i64>().ok()) {
                        row.insert("pid".into(), json!(pid));
                    }
                    apps.push(row);
                }
            }
            Ok(apps)
        }
        _ => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod list_apps_tests {
    use super::super::mcp::McpToolResult;
    use super::*;

    #[test]
    fn parse_list_apps_from_text_lines() {
        let out = McpToolResult {
            data: serde_json::json!("Safari (pid 123)\nGoogle Chrome (pid 456)\n"),
            images: vec![],
            structured: None,
            is_error: false,
        };
        let apps = parse_list_apps(&out).expect("parse");
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0]["name"], "Safari");
        assert_eq!(apps[0]["pid"], 123);
    }

    #[test]
    fn parse_key_combo_normalizes_return_for_cua_driver() {
        let (key, mods) = parse_key_combo("Return");
        assert_eq!(key.as_deref(), Some("return"));
        assert!(mods.is_empty());
        let (key2, _) = parse_key_combo("cmd+Return");
        assert_eq!(key2.as_deref(), Some("return"));
    }

    #[test]
    fn transient_daemon_errors_are_classified() {
        // The exact message observed from cua-driver 0.2.0 in the failing session.
        assert!(CuaDriverBackend::is_transient_daemon_error(
            "Internal error: daemon transport: daemon closed connection"
        ));
        assert!(CuaDriverBackend::is_transient_daemon_error(
            "cua-driver request 'click' timed out after 60s (daemon unresponsive)"
        ));
        assert!(CuaDriverBackend::is_transient_daemon_error("broken pipe"));
        // Real argument/validation errors must NOT trigger a reconnect-retry.
        assert!(!CuaDriverBackend::is_transient_daemon_error(
            "element_index 99 out of range"
        ));
        assert!(!CuaDriverBackend::is_transient_daemon_error(
            "No active window"
        ));
    }

    #[test]
    fn type_without_element_ignores_last_element_for_hermes_pid_typing() {
        let mut backend = CuaDriverBackend::new("cua-driver");
        backend.last_element = Some(7);
        backend.element_scope_pid = Some(100);
        backend.element_scope_window_id = Some(200);
        backend.active_pid = Some(100);
        backend.active_window_id = Some(200);
        backend.element_scope_max_index = Some(10);
        assert_eq!(
            backend.resolve_type_element_index(None).expect("resolve"),
            None
        );
        assert_eq!(
            backend
                .resolve_type_element_index(Some(3))
                .expect("explicit"),
            Some(3)
        );
    }

    #[test]
    fn parse_gws_structured_content_tree() {
        let gws = McpToolResult {
            data: serde_json::json!("✅ Safari — 5 elements\n"),
            images: vec![],
            structured: Some(serde_json::json!({
                "tree_markdown": "- AXApplication \"Safari\"\n  - [0] AXTextField \"Search\"",
                "element_count": 5,
                "screenshot_width": 1200,
                "screenshot_height": 800
            })),
            is_error: false,
        };
        let parsed = parse_gws_result(&gws);
        assert!(parsed.tree.contains("[0] AXTextField"));
        assert_eq!(parsed.width, 1200);
        assert_eq!(parsed.height, 800);
        assert_eq!(parsed.element_count, Some(5));
        let elements = parse_elements(&parsed.tree);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].role, "AXTextField");
    }

    #[test]
    fn parse_gws_json_object_data() {
        let gws = McpToolResult {
            data: serde_json::json!({
                "tree_markdown": "- [1] AXButton",
                "element_count": 1,
                "screenshot_width": 640,
                "screenshot_height": 480
            }),
            images: vec![],
            structured: None,
            is_error: false,
        };
        let parsed = parse_gws_result(&gws);
        assert_eq!(parsed.tree, "- [1] AXButton");
        assert_eq!(parsed.width, 640);
        assert_eq!(parse_elements(&parsed.tree).len(), 1);
    }

    #[test]
    fn parse_elements_extracts_actions_array() {
        let tree = "\
- AXApplication \"Safari\"\n\
  - [0] AXButton \"Reload\" actions=[AXPress]\n\
  - [1] AXMenuBarItem \"Apple\" actions=[AXCancel, AXPick]\n\
  - [2] AXButton actions=[AXShowMenu, AXPress]\n\
  - [3] AXLink \"download\" actions=[AXOpen]\n";
        let elements = parse_elements(tree);
        assert_eq!(elements.len(), 4);
        assert_eq!(elements[0].actions, vec!["AXPress"]);
        assert_eq!(elements[1].actions, vec!["AXCancel", "AXPick"]);
        assert_eq!(elements[2].actions, vec!["AXShowMenu", "AXPress"]);
        assert_eq!(elements[3].actions, vec!["AXOpen"]);
    }

    #[test]
    fn parse_actions_handles_no_actions() {
        let line = "- [5] AXGroup \"container\"";
        assert!(parse_actions_for_line(line).is_none());
    }

    #[test]
    fn parse_gws_legacy_text_tree() {
        let gws = McpToolResult {
            data: serde_json::json!(
                "✅ Terminal — 2 elements\n- AXApplication \"Terminal\"\n  - [0] AXButton"
            ),
            images: vec![],
            structured: None,
            is_error: false,
        };
        let parsed = parse_gws_result(&gws);
        assert!(parsed.summary.contains("Terminal"));
        assert_eq!(parse_elements(&parsed.tree).len(), 1);
    }

    #[test]
    fn app_match_filters_bundle_id_to_localized_name() {
        let filters = app_match_filters("com.apple.Safari");
        assert!(filters.iter().any(|f| f == "Safari"));
    }

    #[test]
    fn pick_window_prefers_on_screen_match() {
        let windows = vec![
            WindowRow {
                app_name: "ExpressVPN".into(),
                pid: 1,
                window_id: 10,
                title: "VPN".into(),
                off_screen: false,
                z_index: 0,
            },
            WindowRow {
                app_name: "Safari".into(),
                pid: 2,
                window_id: 20,
                title: "Start Page".into(),
                off_screen: false,
                z_index: 5,
            },
        ];
        let picked = pick_window_for_app(&windows, "Safari").expect("safari");
        assert_eq!(picked.app_name, "Safari");
        assert_eq!(picked.pid, 2);
    }

    #[test]
    fn validate_element_scope_rejects_cross_app_index() {
        let err = validate_element_scope(
            56,
            Some(3899),
            Some(122982),
            Some(80),
            Some(81076),
            Some(113658),
            Some("Terminal"),
        )
        .expect_err("cross-app");
        assert!(err.contains("do not transfer"), "got: {err}");
        assert!(err.contains("56"), "got: {err}");
    }

    #[test]
    fn validate_element_scope_rejects_out_of_range() {
        let err = validate_element_scope(
            99,
            Some(1),
            Some(2),
            Some(10),
            Some(1),
            Some(2),
            Some("Notes"),
        )
        .expect_err("range");
        assert!(err.contains("out of range"), "got: {err}");
    }

    #[test]
    fn pick_window_resolves_chrome_bundle_alias() {
        let windows = vec![WindowRow {
            app_name: "Google Chrome".into(),
            pid: 99,
            window_id: 1,
            title: String::new(),
            off_screen: false,
            z_index: 0,
        }];
        let picked = pick_window_for_app(&windows, "com.google.Chrome").expect("chrome");
        assert_eq!(picked.pid, 99);
    }
}
