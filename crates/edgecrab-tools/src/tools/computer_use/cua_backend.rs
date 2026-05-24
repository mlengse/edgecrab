//! Cua-driver backend (macOS only) — port of Hermes `cua_backend.py`.

use std::collections::HashMap;

use async_trait::async_trait;
use regex::Regex;
use serde_json::json;

use super::backend::{ActionResult, CaptureResult, ComputerUseBackend, UIElement};
use super::mcp::CuaMcpSession;
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
            r#"^\s*(?:-\s+)?\[(\d+)\]\s+(\w+)(?:\s+"([^"]*)"|(?:\s+\(\d+\))?\s+id=([^\s\[\]]*))?"#,
        )
        .expect("regex")
    })
}

pub struct CuaDriverBackend {
    cmd: String,
    session: Option<CuaMcpSession>,
    active_pid: Option<i32>,
    active_window_id: Option<i32>,
    last_app: Option<String>,
}

impl CuaDriverBackend {
    pub fn new(cmd: impl Into<String>) -> Self {
        Self {
            cmd: cmd.into(),
            session: None,
            active_pid: None,
            active_window_id: None,
            last_app: None,
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
        let session = self.session.as_mut().ok_or("backend not started")?;
        let lw = session
            .call_tool("list_windows", json!({ "on_screen_only": true }))
            .await?;

        let mut windows = parse_windows(&lw);
        if windows.is_empty() {
            return Ok(empty_capture(mode));
        }

        if let Some(filter) = app {
            let fl = filter.to_ascii_lowercase();
            windows.retain(|w| w.app_name.to_ascii_lowercase().contains(&fl));
            if windows.is_empty() {
                return Ok(CaptureResult {
                    mode: mode.into(),
                    width: 0,
                    height: 0,
                    png_b64: None,
                    elements: vec![],
                    app: String::new(),
                    window_title: format!(
                        "<no on-screen window matched app={filter:?}; call list_apps for localized names>"
                    ),
                    png_bytes_len: 0,
                });
            }
        }

        let target = windows
            .iter()
            .find(|w| !w.off_screen)
            .or(windows.first())
            .cloned()
            .ok_or("no window")?;

        self.active_pid = Some(target.pid);
        self.active_window_id = Some(target.window_id);
        if app.is_some() || self.last_app.is_none() {
            self.last_app = Some(target.app_name.clone());
        }

        let mut png_b64 = None;
        let mut elements = Vec::new();
        let mut window_title = String::new();

        if mode == "vision" {
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
        } else {
            let gws = session
                .call_tool(
                    "get_window_state",
                    json!({ "pid": target.pid, "window_id": target.window_id }),
                )
                .await?;
            let text = gws.data.as_str().unwrap_or("").to_string();
            let (summary, tree) = split_tree(&text);
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
                window_title = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            }
            let _ = summary;
        }

        let png_bytes_len = png_b64
            .as_ref()
            .map(|b| b.len() * 3 / 4)
            .unwrap_or(0);

        Ok(CaptureResult {
            mode: mode.into(),
            width: 0,
            height: 0,
            png_b64,
            elements,
            app: target.app_name,
            window_title,
            png_bytes_len,
        })
    }

    async fn click(
        &mut self,
        element: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        button: &str,
        click_count: u32,
        modifiers: Option<&[String]>,
    ) -> Result<ActionResult, String> {
        let pid = self.active_pid.ok_or("No active window — call capture() first.")?;
        let tool = match (button, click_count) {
            ("right", _) => "right_click",
            (_, 2) => "double_click",
            _ => "click",
        };
        let mut args = json!({ "pid": pid });
        if let Some(el) = element {
            let wid = self
                .active_window_id
                .ok_or("No active window_id for element_index click.")?;
            args["element_index"] = json!(el);
            args["window_id"] = json!(wid);
        } else if let (Some(x), Some(y)) = (x, y) {
            args["x"] = json!(x);
            args["y"] = json!(y);
        } else {
            return Err(format!("{tool} requires element= or x/y."));
        }
        if let Some(mods) = modifiers {
            args["modifier"] = json!(mods);
        }
        self.action(tool, args).await
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
        let pid = self.active_pid.ok_or("No active window — call capture() first.")?;
        let mut args = json!({ "pid": pid });
        if let (Some(f), Some(t)) = (from_element, to_element) {
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
            return Err("drag requires from_element/to_element or from_coordinate/to_coordinate.".into());
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
        let pid = self.active_pid.ok_or("No active window — call capture() first.")?;
        let mut args = json!({
            "pid": pid,
            "direction": direction,
            "amount": amount.clamp(1, 50)
        });
        if let Some(el) = element {
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

    async fn type_text(&mut self, text: &str) -> Result<ActionResult, String> {
        let pid = self.active_pid.ok_or("No active window — call capture() first.")?;
        self.action("type_text", json!({ "pid": pid, "text": text }))
            .await
    }

    async fn key(&mut self, keys: &str) -> Result<ActionResult, String> {
        let pid = self.active_pid.ok_or("No active window — call capture() first.")?;
        let (key, mods) = parse_key_combo(keys);
        let key = key.ok_or_else(|| format!("Could not parse key from '{keys}'."))?;
        if mods.is_empty() {
            self.action("press_key", json!({ "pid": pid, "key": key }))
                .await
        } else {
            self.action(
                "hotkey",
                json!({ "pid": pid, "keys": mods.into_iter().chain([key]).collect::<Vec<_>>() }),
            )
            .await
        }
    }

    async fn list_apps(&mut self) -> Result<Vec<HashMap<String, serde_json::Value>>, String> {
        let session = self.session.as_mut().ok_or("backend not started")?;
        let out = session.call_tool("list_apps", json!({})).await?;
        if let Some(arr) = out.data.as_array() {
            return Ok(arr
                .iter()
                .filter_map(|v| v.as_object().map(|o| {
                    o.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                }))
                .collect());
        }
        Ok(Vec::new())
    }

    async fn focus_app(&mut self, app: &str, _raise_window: bool) -> Result<ActionResult, String> {
        let session = self.session.as_mut().ok_or("backend not started")?;
        let lw = session
            .call_tool("list_windows", json!({ "on_screen_only": true }))
            .await?;
        let windows = parse_windows(&lw);
        let fl = app.to_ascii_lowercase();
        let target = windows
            .iter()
            .find(|w| w.app_name.to_ascii_lowercase().contains(&fl));
        if let Some(t) = target {
            self.active_pid = Some(t.pid);
            self.active_window_id = Some(t.window_id);
            self.last_app = Some(t.app_name.clone());
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
        Err(format!("No on-screen window found for app '{app}'."))
    }

    async fn set_value(&mut self, value: &str, element: Option<u32>) -> Result<ActionResult, String> {
        let pid = self.active_pid.ok_or("No active window — call capture() first.")?;
        let wid = self
            .active_window_id
            .ok_or("No active window — call capture() first.")?;
        let el = element.ok_or("set_value requires element= (element index).")?;
        self.action(
            "set_value",
            json!({ "pid": pid, "window_id": wid, "element_index": el, "value": value }),
        )
        .await
    }
}

impl CuaDriverBackend {
    async fn action(&mut self, name: &str, args: serde_json::Value) -> Result<ActionResult, String> {
        let session = self.session.as_mut().ok_or("backend not started")?;
        let out = session.call_tool(name, args).await?;
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

    #[allow(dead_code)]
    pub fn last_app(&self) -> Option<&str> {
        self.last_app.as_deref()
    }
}

struct WindowRow {
    app_name: String,
    pid: i32,
    window_id: i32,
    off_screen: bool,
}

impl Clone for WindowRow {
    fn clone(&self) -> Self {
        Self {
            app_name: self.app_name.clone(),
            pid: self.pid,
            window_id: self.window_id,
            off_screen: self.off_screen,
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
                    off_screen: !w.get("is_on_screen").and_then(|v| v.as_bool()).unwrap_or(true),
                })
            })
            .collect();
        windows.sort_by_key(|w| w.window_id);
        return windows;
    }
    let text = lw.data.as_str().unwrap_or("");
    window_line_re()
        .captures_iter(text)
        .map(|cap| WindowRow {
            app_name: cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
            pid: cap.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            window_id: cap.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            off_screen: cap.get(0).map(|m| m.as_str().contains("[off-screen]")).unwrap_or(false),
        })
        .collect()
}

fn split_tree(full: &str) -> (String, String) {
    match full.split_once('\n') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (full.to_string(), String::new()),
    }
}

fn parse_elements(tree: &str) -> Vec<UIElement> {
    element_line_re()
        .captures_iter(tree)
        .map(|cap| UIElement {
            index: cap.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            role: cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default(),
            label: cap
                .get(3)
                .or_else(|| cap.get(4))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default(),
            bounds: (0, 0, 0, 0),
            app: String::new(),
        })
        .collect()
}

fn parse_key_combo(keys: &str) -> (Option<String>, Vec<String>) {
    const MODS: &[&str] = &["cmd", "command", "shift", "option", "alt", "ctrl", "control", "fn"];
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
            key = Some(part.trim().to_string());
        }
    }
    (key, modifiers)
}

fn empty_capture(mode: &str) -> CaptureResult {
    CaptureResult {
        mode: mode.into(),
        width: 0,
        height: 0,
        png_b64: None,
        elements: vec![],
        app: String::new(),
        window_title: String::new(),
        png_bytes_len: 0,
    }
}
