use crate::auth::{now_ms, user_by_token, AuthUser};
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::models::{ClientMessage, FileMeta, Message as ChatMessage, MessageQuery, WsQuery};
use crate::state::AppState;
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::{interval, MissedTickBehavior};
use uuid::Uuid;

/// 历史消息：before 传当前列表最早一条的 key，向上翻页。
pub async fn history(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<MessageQuery>,
) -> ApiResult<Json<Value>> {
    let limit = query.limit.unwrap_or(50);
    let (messages, has_more) =
        db::list_messages(state.db.as_ref(), query.before.as_deref(), limit)?;
    Ok(Json(json!({ "messages": messages, "has_more": has_more })))
}

/// 浏览器 WebSocket 无法自定义 header，token 通过 query 传递。
pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> ApiResult<Response> {
    let user = user_by_token(&state, &query.token)?;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user)))
}

async fn handle_socket(socket: WebSocket, state: AppState, user: crate::models::UserRecord) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.hub.subscribe();
    let mut ping = interval(Duration::from_secs(25));
    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
    tracing::info!(username = %user.username, "客户端接入大厅");

    loop {
        tokio::select! {
            broadcast = rx.recv() => {
                if let Ok(text) = broadcast {
                    if sender.send(WsMessage::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(WsMessage::Text(text))) => {
                        if let Err(err) = handle_client_message(&state, &user, &text).await {
                            let payload =
                                json!({ "type": "error", "error": err.message() }).to_string();
                            let _ = sender.send(WsMessage::Text(payload)).await;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            _ = ping.tick() => {
                if sender.send(WsMessage::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        }
    }
    tracing::info!(username = %user.username, "客户端离开大厅");
}

async fn handle_client_message(
    state: &AppState,
    user: &crate::models::UserRecord,
    text: &str,
) -> ApiResult<()> {
    let req: ClientMessage =
        serde_json::from_str(text).map_err(|_| ApiError::bad_request("消息格式错误"))?;
    let config = state.config_snapshot();
    let id = Uuid::new_v4().to_string();
    let ts = now_ms();
    let key = db::message_key(ts, &id);
    let mut message = ChatMessage {
        id,
        key,
        ts,
        username: user.username.clone(),
        kind: String::new(),
        text: String::new(),
        file: None,
    };

    match req.kind.as_str() {
        "text" => {
            let body = req.text.trim();
            if body.is_empty() {
                return Err(ApiError::bad_request("消息不能为空"));
            }
            if body.chars().count() > config.max_message_length {
                return Err(ApiError::bad_request("消息过长"));
            }
            message.kind = "text".to_string();
            message.text = body.to_string();
        }
        "file" => {
            let meta = db::get_json::<FileMeta>(state.db.as_ref(), db::FILES, &req.file_id)?
                .ok_or_else(|| ApiError::not_found("文件不存在"))?;
            message.kind = "file".to_string();
            message.file = Some(meta);
        }
        _ => return Err(ApiError::bad_request("未知消息类型")),
    }

    db::put_json(state.db.as_ref(), db::MESSAGES, &message.key, &message)?;
    let payload = serde_json::to_string(&message).map_err(ApiError::internal)?;
    // 没有在线接收者时消息已落库，不算错误
    let _ = state.hub.send(payload);
    Ok(())
}
