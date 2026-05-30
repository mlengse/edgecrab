//! Action dispatch for `computer_use`.

use serde_json::json;

use super::backend::ComputerUseBackend;
use super::browsers::{is_browser_app, should_open_url_via_launch};
use super::response::{action_response, finalize_capture_response, max_elements_from_args};
use super::text_input::{is_address_bar_focus_combo, is_submit_key};
use super::text_input::looks_like_url_or_domain;
use crate::registry::ToolContext;

pub async fn dispatch_action(
    backend: &mut dyn ComputerUseBackend,
    action: &str,
    args: &serde_json::Value,
    edgecrab_home: &std::path::Path,
    tool_ctx: Option<&ToolContext>,
) -> Result<String, String> {
    let capture_after = args.get("capture_after").and_then(|v| v.as_bool()) == Some(true);
    let action_app = args.get("app").and_then(|v| v.as_str());

    match action {
        "capture" => {
            let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("som");
            if !matches!(mode, "som" | "vision" | "ax") {
                Err(format!("bad mode {mode:?}; use som|vision|ax"))
            } else {
                let cap = backend
                    .capture_with_query(
                        mode,
                        args.get("app").and_then(|v| v.as_str()),
                        args.get("query").and_then(|v| v.as_str()),
                    )
                    .await?;
                finalize_capture_response(
                    &cap,
                    max_elements_from_args(args),
                    edgecrab_home,
                    tool_ctx,
                )
                .await
            }
        }
        "wait" => {
            let seconds = args.get("seconds").and_then(|v| v.as_f64()).unwrap_or(1.0);
            Ok(action_response(&backend.wait(seconds).await))
        }
        "list_apps" => {
            let apps = backend.list_apps().await?;
            if apps.is_empty()
                && !std::env::var("EDGECRAB_COMPUTER_USE_BACKEND")
                    .map(|v| v.eq_ignore_ascii_case("noop"))
                    .unwrap_or(false)
            {
                return Ok(json!({
                    "apps": [],
                    "count": 0,
                    "error": "No applications visible to cua-driver",
                    "hint": super::permissions::permissions_failure_hint(),
                    "next_steps": [
                        "Run /computer open and grant Screen Recording + Accessibility",
                        "Quit and relaunch EdgeCrab after granting permissions",
                        "Open Safari (or target app) so a window is on screen",
                        "Retry list_apps, then focus_app app=Safari"
                    ]
                })
                .to_string());
            }
            Ok(json!({ "apps": apps, "count": apps.len() }).to_string())
        }
        "focus_app" => {
            let app = args
                .get("app")
                .and_then(|v| v.as_str())
                .ok_or("focus_app requires `app`")?;
            let res = backend
                .focus_app(app, args.get("raise_window").and_then(|v| v.as_bool()) == Some(true))
                .await?;
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        "launch_app" => {
            // Accept `bundle_id`, `app`, or `name` to match cua-driver naming
            // and the rest of our schema's `app=` convention.
            let target = args
                .get("bundle_id")
                .or_else(|| args.get("app"))
                .or_else(|| args.get("name"))
                .and_then(|v| v.as_str())
                .ok_or("launch_app requires `bundle_id`, `app`, or `name`")?;
            let urls: Option<Vec<String>> = args
                .get("urls")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            let res = backend.launch_app(target, urls.as_deref()).await?;
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        "click" | "double_click" | "right_click" | "middle_click" => {
            let (button, click_count) = match action {
                "double_click" => ("left", 2u32),
                "right_click" => ("right", 1),
                "middle_click" => ("middle", 1),
                _ => (
                    args.get("button").and_then(|v| v.as_str()).unwrap_or("left"),
                    1,
                ),
            };
            let coord = args.get("coordinate").and_then(|v| v.as_array());
            let (x, y) = match coord {
                Some(c) if c.len() >= 2 => (
                    c[0].as_i64().map(|v| v as i32),
                    c[1].as_i64().map(|v| v as i32),
                ),
                _ => (None, None),
            };
            let mods = modifiers(args);
            let ax_action = args.get("ax_action").and_then(|v| v.as_str());
            backend.prepare_action_target(action_app).await?;
            let res = backend
                .click(
                    args.get("element").and_then(|v| v.as_u64()).map(|v| v as u32),
                    x,
                    y,
                    button,
                    click_count,
                    mods.as_deref(),
                    ax_action,
                )
                .await?;
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        "drag" => {
            let has_el = args.get("from_element").is_some() && args.get("to_element").is_some();
            let has_xy = args.get("from_coordinate").is_some() && args.get("to_coordinate").is_some();
            if !has_el && !has_xy {
                Err(
                    "drag requires from_coordinate/to_coordinate or from_element/to_element".into(),
                )
            } else {
                let from_xy = coord_pair(args.get("from_coordinate"));
                let to_xy = coord_pair(args.get("to_coordinate"));
                backend.prepare_action_target(action_app).await?;
                let res = backend
                    .drag(
                        args.get("from_element").and_then(|v| v.as_u64()).map(|v| v as u32),
                        args.get("to_element").and_then(|v| v.as_u64()).map(|v| v as u32),
                        from_xy,
                        to_xy,
                        args.get("button").and_then(|v| v.as_str()).unwrap_or("left"),
                        modifiers(args).as_deref(),
                    )
                    .await?;
                maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
            }
        }
        "scroll" => {
            let coord = coord_pair(args.get("coordinate"));
            backend.prepare_action_target(action_app).await?;
            let res = backend
                .scroll(
                    args.get("direction").and_then(|v| v.as_str()).unwrap_or("down"),
                    args.get("amount").and_then(|v| v.as_i64()).unwrap_or(3) as i32,
                    args.get("element").and_then(|v| v.as_u64()).map(|v| v as u32),
                    coord.map(|(x, _)| x),
                    coord.map(|(_, y)| y),
                    modifiers(args).as_deref(),
                )
                .await?;
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        "navigate" => {
            let url = args
                .get("url")
                .or_else(|| args.get("text"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .ok_or("navigate requires `url` (or `text`)")?;
            let bundle_id = args.get("bundle_id").and_then(|v| v.as_str());
            backend.prepare_action_target(action_app).await?;
            let res = backend
                .open_browser_url(action_app, bundle_id, url, "navigate")
                .await?;
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        "type" => {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let bundle_id = args.get("bundle_id").and_then(|v| v.as_str());
            backend.prepare_action_target(action_app).await?;
            let app_ctx = action_app.or(backend.targeted_app());
            let res = if should_open_url_via_launch(app_ctx, backend.targeted_app(), text) {
                backend
                    .open_browser_url(action_app, bundle_id, text, "type")
                    .await?
            } else {
                if app_ctx.is_some_and(is_browser_app) && looks_like_url_or_domain(text) {
                    backend.set_pending_browser_url(text);
                }
                backend.type_text(text, None).await?
            };
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        "key" => {
            let keys = args.get("keys").and_then(|v| v.as_str()).unwrap_or("");
            let bundle_id = args.get("bundle_id").and_then(|v| v.as_str());
            backend.prepare_action_target(action_app).await?;
            let app_ctx = action_app.or(backend.targeted_app());
            if is_submit_key(keys) && app_ctx.is_some_and(is_browser_app) {
                if let Some(url) = backend.take_pending_browser_url() {
                    let res = backend
                        .open_browser_url(action_app, bundle_id, &url, "key")
                        .await?;
                    return maybe_follow_capture(
                        backend,
                        res,
                        capture_after,
                        args,
                        edgecrab_home,
                        tool_ctx,
                    )
                    .await;
                }
                let res = super::backend::ActionResult {
                    ok: false,
                    action: "key".into(),
                    message: "Return does not commit browser navigation in background (cua-driver). \
                        Use action=navigate(url='https://x.com', app='Google Chrome') or \
                        launch_app(bundle_id='com.google.Chrome', urls=['https://x.com']). \
                        If you already typed a URL, call navigate with that URL instead of Return."
                        .into(),
                    meta: std::collections::HashMap::new(),
                };
                return maybe_follow_capture(
                    backend,
                    res,
                    capture_after,
                    args,
                    edgecrab_home,
                    tool_ctx,
                )
                .await;
            }
            if is_address_bar_focus_combo(keys)
                && is_browser_app(action_app.or(backend.targeted_app()).unwrap_or(""))
            {
                let res = super::backend::ActionResult {
                    ok: false,
                    action: "key".into(),
                    message: "cmd+l blocked for browsers — use action=navigate(url='https://…', app='Google Chrome') \
                        or launch_app(bundle_id='com.google.Chrome', urls=['https://…']). \
                        Omnibox typing leaves the old URL selected and Return does not navigate."
                        .into(),
                    meta: std::collections::HashMap::new(),
                };
                return maybe_follow_capture(
                    backend,
                    res,
                    capture_after,
                    args,
                    edgecrab_home,
                    tool_ctx,
                )
                .await;
            }
            let res = backend.key(keys).await?;
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        "set_value" => {
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or("set_value requires `value`")?;
            backend.prepare_action_target(action_app).await?;
            let res = backend
                .set_value(
                    value,
                    args.get("element").and_then(|v| v.as_u64()).map(|v| v as u32),
                )
                .await?;
            maybe_follow_capture(backend, res, capture_after, args, edgecrab_home, tool_ctx).await
        }
        other => Err(format!("unknown action {other:?}")),
    }
}

pub(crate) async fn maybe_follow_capture(
    backend: &mut dyn ComputerUseBackend,
    res: super::backend::ActionResult,
    do_capture: bool,
    args: &serde_json::Value,
    edgecrab_home: &std::path::Path,
    tool_ctx: Option<&ToolContext>,
) -> Result<String, String> {
    if !do_capture || !res.ok {
        return Ok(action_response(&res));
    }
    let app = backend.targeted_app().map(str::to_string);
    let cap = backend.capture("som", app.as_deref()).await?;
    let max_elements = max_elements_from_args(args);
    let resp = finalize_capture_response(&cap, max_elements, edgecrab_home, tool_ctx).await?;
    let first_line = resp.lines().next().unwrap_or("");
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(first_line)
        && value.get("_multimodal") == Some(&json!(true))
    {
        let prefix = action_prefix(&res);
        if let Some(content) = value.get_mut("content").and_then(|c| c.as_array_mut())
            && let Some(first) = content.first_mut()
            && let Some(text) = first.get_mut("text")
        {
            *text = json!(format!("{prefix}\n\n{}", text.as_str().unwrap_or("")));
        }
        if let Some(summary) = value.get_mut("text_summary") {
            *summary = json!(format!(
                "{prefix}\n\n{}",
                summary.as_str().unwrap_or("")
            ));
        }
        Ok(serde_json::to_string(&value).unwrap_or(resp))
    } else if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&resp) {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("prior_action".into(), json!(res.action));
            obj.insert("prior_ok".into(), json!(res.ok));
            if !res.message.is_empty() {
                obj.insert("prior_message".into(), json!(res.message));
            }
        }
        Ok(serde_json::to_string(&value).unwrap_or(resp))
    } else {
        Ok(format!("{}\n{}", action_response(&res), resp))
    }
}

fn action_prefix(res: &super::backend::ActionResult) -> String {
    format!(
        "[{}] ok={}{}",
        res.action,
        res.ok,
        if res.message.is_empty() {
            String::new()
        } else {
            format!(" — {}", res.message)
        }
    )
}

fn coord_pair(value: Option<&serde_json::Value>) -> Option<(i32, i32)> {
    let arr = value?.as_array()?;
    if arr.len() < 2 {
        return None;
    }
    Some((
        arr[0].as_i64()? as i32,
        arr[1].as_i64()? as i32,
    ))
}

fn modifiers(args: &serde_json::Value) -> Option<Vec<String>> {
    args.get("modifiers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<String>>()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
}
