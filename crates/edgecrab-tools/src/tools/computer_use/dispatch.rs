//! Action dispatch for `computer_use`.

use serde_json::json;

use super::backend::ComputerUseBackend;
use super::response::{action_response, finalize_capture_response, max_elements_from_args};
use crate::registry::ToolContext;

pub async fn dispatch_action(
    backend: &mut dyn ComputerUseBackend,
    action: &str,
    args: &serde_json::Value,
    edgecrab_home: &std::path::Path,
    tool_ctx: Option<&ToolContext>,
) -> Result<String, String> {
    let capture_after = args.get("capture_after").and_then(|v| v.as_bool()) == Some(true);

    match action {
        "capture" => {
            let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("som");
            if !matches!(mode, "som" | "vision" | "ax") {
                Err(format!("bad mode {mode:?}; use som|vision|ax"))
            } else {
                let cap = backend
                    .capture(mode, args.get("app").and_then(|v| v.as_str()))
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
            maybe_follow_capture(backend, res, capture_after, edgecrab_home, tool_ctx).await
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
            let res = backend
                .click(
                    args.get("element").and_then(|v| v.as_u64()).map(|v| v as u32),
                    x,
                    y,
                    button,
                    click_count,
                    mods.as_deref(),
                )
                .await?;
            maybe_follow_capture(backend, res, capture_after, edgecrab_home, tool_ctx).await
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
                maybe_follow_capture(backend, res, capture_after, edgecrab_home, tool_ctx).await
            }
        }
        "scroll" => {
            let coord = coord_pair(args.get("coordinate"));
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
            maybe_follow_capture(backend, res, capture_after, edgecrab_home, tool_ctx).await
        }
        "type" => {
            let res = backend
                .type_text(args.get("text").and_then(|v| v.as_str()).unwrap_or(""))
                .await?;
            maybe_follow_capture(backend, res, capture_after, edgecrab_home, tool_ctx).await
        }
        "key" => {
            let res = backend
                .key(args.get("keys").and_then(|v| v.as_str()).unwrap_or(""))
                .await?;
            maybe_follow_capture(backend, res, capture_after, edgecrab_home, tool_ctx).await
        }
        "set_value" => {
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or("set_value requires `value`")?;
            let res = backend
                .set_value(
                    value,
                    args.get("element").and_then(|v| v.as_u64()).map(|v| v as u32),
                )
                .await?;
            maybe_follow_capture(backend, res, capture_after, edgecrab_home, tool_ctx).await
        }
        other => Err(format!("unknown action {other:?}")),
    }
}

async fn maybe_follow_capture(
    backend: &mut dyn ComputerUseBackend,
    res: super::backend::ActionResult,
    do_capture: bool,
    edgecrab_home: &std::path::Path,
    tool_ctx: Option<&ToolContext>,
) -> Result<String, String> {
    if !do_capture || !res.ok {
        return Ok(action_response(&res));
    }
    let cap = backend.capture("som", None).await?;
    let resp = finalize_capture_response(
        &cap,
        super::schema::DEFAULT_MAX_ELEMENTS,
        edgecrab_home,
        tool_ctx,
    )
    .await?;
    let first_line = resp.lines().next().unwrap_or("");
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(first_line)
        && value.get("_multimodal") == Some(&json!(true))
    {
        let prefix = format!(
            "[{}] ok={}{}",
            res.action,
            res.ok,
            if res.message.is_empty() {
                String::new()
            } else {
                format!(" — {}", res.message)
            }
        );
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
    } else {
        Ok(resp)
    }
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
