//! Read-only Kanban HTTP + WebSocket routes — Hermes dashboard API seed.

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use edgecrab_core::{edgecrab_home, kanban_api, AppConfig};
use serde::Deserialize;
use serde_json::{json, Value};

/// Hermes-compatible poll interval for the event tail loop.
const EVENT_POLL_MS: u64 = 300;

#[derive(Debug, Deserialize, Default)]
pub struct BoardQuery {
    pub board: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct EventsQuery {
    pub board: Option<String>,
    #[serde(default)]
    pub since: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

fn kanban_disabled() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "kanban disabled" })),
    )
}

fn api_err(e: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": e.to_string() })),
    )
}

fn not_found(msg: impl Into<String>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": msg.into() })),
    )
}

fn kanban_enabled() -> bool {
    AppConfig::load()
        .map(|c| c.kanban.enabled)
        .unwrap_or(false)
}

fn events_params(params: &EventsQuery) -> (Option<String>, i64, usize) {
    let since = params.since.unwrap_or(0).max(0);
    let limit = params.limit.unwrap_or(200).clamp(1, 500);
    (params.board.clone(), since, limit)
}

/// `GET /api/kanban/board?board=<slug>`
pub async fn kanban_board(
    Query(params): Query<BoardQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !kanban_enabled() {
        return Err(kanban_disabled());
    }
    kanban_api::board_snapshot(Some(&edgecrab_home()), params.board.as_deref())
        .map(Json)
        .map_err(api_err)
}

/// `GET /api/kanban/boards`
pub async fn kanban_boards() -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !kanban_enabled() {
        return Err(kanban_disabled());
    }
    kanban_api::boards_list(Some(&edgecrab_home()))
        .map(Json)
        .map_err(api_err)
}

/// `GET /api/kanban/tasks/:id?board=<slug>`
pub async fn kanban_task_detail(
    Path(task_id): Path<String>,
    Query(params): Query<BoardQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !kanban_enabled() {
        return Err(kanban_disabled());
    }
    match kanban_api::task_detail(
        Some(&edgecrab_home()),
        params.board.as_deref(),
        &task_id,
    ) {
        Ok(v) => Ok(Json(v)),
        Err(edgecrab_types::AgentError::Validation(msg)) => Err(not_found(msg)),
        Err(e) => Err(api_err(e)),
    }
}

/// Minimal kanban board UI — `GET /kanban`.
pub async fn kanban_dashboard() -> Result<axum::response::Html<&'static str>, (StatusCode, Json<Value>)> {
    if !kanban_enabled() {
        return Err(kanban_disabled());
    }
    Ok(axum::response::Html(include_str!("kanban_dashboard.html")))
}

/// `GET /api/kanban/events?since=<id>&board=<slug>&limit=<n>`
pub async fn kanban_events_poll(
    Query(params): Query<EventsQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !kanban_enabled() {
        return Err(kanban_disabled());
    }
    let (board, since, limit) = events_params(&params);
    kanban_api::events_since(
        Some(&edgecrab_home()),
        board.as_deref(),
        since,
        limit,
    )
    .map(Json)
    .map_err(api_err)
}

/// `GET /api/kanban/events/ws?since=<id>&board=<slug>` — Hermes `/events` tail.
pub async fn kanban_events_ws(
    Query(params): Query<EventsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    if !kanban_enabled() {
        return kanban_disabled().into_response();
    }
    let (board, since, limit) = events_params(&params);
    ws.on_upgrade(move |socket| kanban_events_stream(socket, board, since, limit))
}

async fn kanban_events_stream(
    mut socket: WebSocket,
    board: Option<String>,
    mut cursor: i64,
    limit: usize,
) {
    let home = edgecrab_home();
    loop {
        let board = board.clone();
        let home = home.clone();
        let fetch = tokio::task::spawn_blocking(move || {
            kanban_api::events_since(Some(&home), board.as_deref(), cursor, limit)
        })
        .await;

        match fetch {
            Ok(Ok(body)) => {
                if let Some(new_cursor) = body.get("cursor").and_then(|v| v.as_i64()) {
                    cursor = new_cursor;
                }
                let has_events = body
                    .get("events")
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| !a.is_empty());
                if has_events {
                    let text = match serde_json::to_string(&body) {
                        Ok(t) => t,
                        Err(_) => break,
                    };
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
            Ok(Err(e)) => {
                let err = json!({ "error": e.to_string() });
                if socket
                    .send(Message::Text(err.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(_) => break,
        }

        tokio::time::sleep(Duration::from_millis(EVENT_POLL_MS)).await;
    }
}
