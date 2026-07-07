/// WebSocket 处理：升级 HTTP 连接并向所有已连接客户端广播消息。
///
/// 从内嵌音频引擎直接获取播放状态（不再通过 HTTP 轮询 C++ 引擎）。
use crate::app::state::{AppState, OnlineListener};
use crate::error::AppError;
use crate::models::WsMessage;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;

/// WebSocket 升级处理器 — GET /ws
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let device_token = params.get("device_token").cloned().unwrap_or_default();
    ws.on_upgrade(move |socket| handle_socket(socket, state, device_token))
}

/// 处理单个 WebSocket 连接的生命周期。
async fn handle_socket(socket: WebSocket, state: Arc<AppState>, device_token: String) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.ws_tx.subscribe();

    // 查询设备用户信息并注册到在线听众列表
    let display_name = if !device_token.is_empty() {
        let user = sqlx::query_as::<_, (String,)>(
            "SELECT display_name FROM device_users WHERE device_token = ?",
        )
        .bind(&device_token)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .map(|(name,)| name)
        .unwrap_or_else(|| "Anonymous".into());

        state.listeners.insert(
            device_token.clone(),
            OnlineListener {
                display_name: user.clone(),
                connected_at: chrono::Utc::now(),
            },
        );

        // 广播在线听众更新
        broadcast_listeners_update(&state);

        user
    } else {
        "Anonymous".into()
    };

    let welcome = serde_json::to_string(&WsMessage::Notice {
        message: format!("Connected to {}", state.config.station.name),
        level: "info".into(),
    })
    .unwrap_or_default();

    if sender.send(Message::Text(welcome.into())).await.is_err() {
        // 连接失败，清理注册
        if !device_token.is_empty() {
            state.listeners.remove(&device_token);
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
    if !device_token.is_empty() {
        state.listeners.remove(&device_token);
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
    let names: Vec<String> = state.listeners.iter().map(|entry| entry.value().display_name.clone()).collect();

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
