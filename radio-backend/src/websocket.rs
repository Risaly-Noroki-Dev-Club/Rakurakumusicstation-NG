/// WebSocket 处理：升级 HTTP 连接并向所有已连接客户端广播消息。
///
/// 从内嵌音频引擎直接获取播放状态（不再通过 HTTP 轮询 C++ 引擎）。
use crate::app::state::{AppState, OnlineListener};
use crate::auth::{self, AuthUser};
use crate::error::AppError;
use crate::models::WsMessage;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

/// WebSocket 升级处理器 — GET /ws
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // New visitors receive a cookie from the global middleware but are not
    // persisted until they use an identity-requiring action. They may listen
    // anonymously, but only known devices affect listener presence.
    let device = match auth::extract_device_token(&headers) {
        Some(device_token) => auth::lookup_device_user(&state.db, &device_token).await?,
        None => None,
    };

    Ok(ws
        .on_upgrade(move |socket| handle_socket(socket, state, device))
        .into_response())
}

/// 处理单个 WebSocket 连接的生命周期。
async fn handle_socket(socket: WebSocket, state: Arc<AppState>, device: Option<AuthUser>) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.ws_tx.subscribe();

    // Only persisted, non-banned devices may appear in listener presence.
    let (device_token, display_name) = if let Some(user) = device {
        let device_token = user.device_token;
        let display_name = user.display_name;
        state.listeners.insert(
            device_token.clone(),
            OnlineListener {
                display_name: display_name.clone(),
                connected_at: chrono::Utc::now(),
            },
        );
        broadcast_listeners_update(&state);
        (Some(device_token), display_name)
    } else {
        (None, "Anonymous".into())
    };

    let welcome = serde_json::to_string(&WsMessage::Notice {
        message: format!("Connected to {}", state.config.station.name),
        level: "info".into(),
    })
    .unwrap_or_default();

    if sender.send(Message::Text(welcome.into())).await.is_err() {
        // 连接失败，清理注册
        if let Some(device_token) = &device_token {
            state.listeners.remove(device_token);
            broadcast_listeners_update(&state);
        }
        return;
    }

    let mut heartbeat = tokio::time::interval(tokio::time::Duration::from_secs(30));
    let mut last_pong = chrono::Utc::now();
    let pong_timeout = chrono::Duration::seconds(60);

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(msg) => {
                        if sender.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged by {} messages", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(t))) if t.trim() == "pong" => {
                        tracing::debug!("Received pong from client");
                        last_pong = chrono::Utc::now();
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(_))) => {},
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }
            _ = heartbeat.tick() => {
                // 检查 pong 超时
                if chrono::Utc::now() - last_pong > pong_timeout {
                    tracing::warn!("WebSocket client pong timeout, disconnecting");
                    break;
                }

                let ping = serde_json::to_string(&WsMessage::Ping {
                    timestamp: chrono::Utc::now().timestamp_millis(),
                }).unwrap_or_default();

                if sender.send(Message::Text(ping.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    // 清理：从在线听众列表移除
    if let Some(device_token) = &device_token {
        state.listeners.remove(device_token);
        broadcast_listeners_update(&state);
    }

    tracing::info!("WebSocket client '{}' disconnected", display_name);
}

/// 广播在线听众更新
fn broadcast_listeners_update(state: &Arc<AppState>) {
    if state.ws_tx.receiver_count() == 0 {
        return;
    }

    let count = state.listeners.len();
    let names: Vec<String> = state
        .listeners
        .iter()
        .map(|entry| entry.value().display_name.clone())
        .collect();

    let msg = WsMessage::ListenersUpdate { count, names };
    let json = serde_json::to_string(&msg).unwrap_or_default();
    let _ = state.ws_tx.send(json);
}

/// 通过广播频道向所有 WebSocket 客户端发布消息。
pub fn broadcast(state: &Arc<AppState>, msg: WsMessage) {
    if state.ws_tx.receiver_count() == 0 {
        return;
    }
    let json = serde_json::to_string(&msg).unwrap_or_default();
    let _ = state.ws_tx.send(json);
}

/// 向音频引擎发送命令（直接调用内嵌引擎）。
pub async fn publish_command(
    state: &Arc<AppState>,
    command: &radio_engine::types::AudioCommand,
) -> Result<(), AppError> {
    state.player_handle.send_command(command.clone());
    Ok(())
}
