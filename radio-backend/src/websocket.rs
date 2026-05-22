/// WebSocket 处理：升级 HTTP 连接并向所有已连接客户端广播消息。
///
/// 从内嵌音频引擎直接获取播放状态（不再通过 HTTP 轮询 C++ 引擎）。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::WsMessage;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

/// WebSocket 升级处理器 — GET /ws
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// 处理单个 WebSocket 连接的生命周期。
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.ws_tx.subscribe();

    let welcome = serde_json::to_string(&WsMessage::Notice {
        message: format!("Connected to {}", state.config.station.name),
        level: "info".into(),
    })
    .unwrap_or_default();

    if sender.send(Message::Text(welcome.into())).await.is_err() {
        return;
    }

    let mut heartbeat = tokio::time::interval(tokio::time::Duration::from_secs(30));

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
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Ping(_))) => {},
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }
            _ = heartbeat.tick() => {
                let ping = serde_json::to_string(&WsMessage::Ping {
                    timestamp: chrono::Utc::now().timestamp_millis(),
                }).unwrap_or_default();

                if sender.send(Message::Text(ping.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    tracing::info!("WebSocket client disconnected");
}

/// 通过广播频道向所有 WebSocket 客户端发布消息。
pub fn broadcast(state: &Arc<AppState>, msg: WsMessage) {
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
