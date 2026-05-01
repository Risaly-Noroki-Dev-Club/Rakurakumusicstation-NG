/// WebSocket 处理：升级 HTTP 连接并向所有已连接客户端广播播放状态。
///
/// 架构：
/// - 在 WebSocket 连接时，处理器订阅 Tokio 广播频道并将消息
///   转发给客户端。
/// - 一个后台任务订阅 Redis `playback_state` 频道，并将消息
///   重新发布到 Tokio 广播，供所有 WebSocket 客户端接收。
/// - 每 30 秒发送一次心跳 ping，以检测断开的连接。

use crate::db::AppState;
use crate::models::WsMessage;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

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

    // 订阅此连接的广播频道
    let mut rx = state.ws_tx.subscribe();

    // 发送初始连接确认
    let welcome = serde_json::to_string(&WsMessage::Notice {
        message: format!("Connected to {}", state.config.station.name),
        level: "info".into(),
    })
    .unwrap_or_default();

    if sender.send(Message::Text(welcome.into())).await.is_err() {
        return;
    }

    // 生成一个任务，将广播消息转发给此客户端
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender
                        .send(Message::Text(msg.into()))
                        .await
                        .is_err()
                    {
                        break; // 客户端已断开连接
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebSocket client lagged by {} messages", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // 处理来自客户端的传入消息（pong 响应等）
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(t) => {
                    // 客户端可以发送 "pong" 来响应心跳
                    if t.trim() == "pong" {
                        tracing::debug!("Received pong from client");
                    }
                }
                Message::Close(_) => break,
                Message::Ping(data) => {
                    // Axum 通过 tungstenite 自动处理 ping/pong
                    let _ = data;
                }
                _ => {}
            }
        }
    });

    // 心跳：每 30 秒发送一次 ping
    let mut heartbeat = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                // 发送 JSON 心跳
                let ping = serde_json::to_string(&WsMessage::Ping {
                    timestamp: chrono::Utc::now().timestamp_millis(),
                }).unwrap_or_default();

                if sender.send(Message::Text(ping.into())).await.is_err() {
                    break;
                }
            }
            _ = &mut send_task => break,
            _ = &mut recv_task => break,
        }
    }

    // 清理
    send_task.abort();
    recv_task.abort();
    tracing::info!("WebSocket client disconnected");
}

/// 通过广播频道向所有 WebSocket 客户端发布消息。
pub fn broadcast(state: &Arc<AppState>, msg: WsMessage) {
    let json = serde_json::to_string(&msg).unwrap_or_default();
    // 仅在无接收者时 Send 返回 Err，这是可接受的。
    let _ = state.ws_tx.send(json);
}

/// 启动后台任务，订阅 Redis `playback_state` 并向 WebSocket 客户端转发消息。
pub async fn start_redis_subscriber(state: Arc<AppState>) {
    let mut pubsub_conn = match redis::aio::ConnectionManager::new(
        redis::Client::open(state.config.redis.url.as_str()).unwrap(),
    )
    .await
    {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to create Redis subscriber connection: {}", e);
            return;
        }
    };

    let channel = state.config.redis.playback_channel.clone();

    tokio::spawn(async move {
        loop {
            // 订阅 playback_state 频道
            // redis-rs 异步 pubsub 需要独立连接，因此我们使用独立
            // 连接进行轮询。
            // 为简单起见，我们使用独立连接的轮询方式。
            //
            // 实际上，我们使用适当的异步 pubsub 模式。
            // 我们将创建一个专用连接用于订阅。
            match redis::Client::open(state.config.redis.url.as_str()) {
                Ok(client) => {
                    match client.get_async_connection().await {
                        Ok(mut conn) => {
                            let mut pubsub = conn.into_pubsub();
                            if pubsub.subscribe(&channel).await.is_ok() {
                                tracing::info!("Subscribed to Redis channel: {}", channel);

                                // 处理消息
                                loop {
                                    match pubsub.on_message().next().await {
                                        Some(msg) => {
                                            let payload: String = msg.get_payload().unwrap_or_default();

                                            // 转发给所有 WebSocket 客户端
                                            let _ = state.ws_tx.send(payload);
                                        }
                                        None => {
                                            tracing::warn!("Redis pubsub stream ended, reconnecting...");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Redis connection error: {}, retrying in 5s...", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Redis client error: {}, retrying in 5s...", e);
                }
            }

            // 重连延迟
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    tracing::info!("Redis subscriber task started for channel: {}", channel);
}

/// 通过 Redis 向 C++ 音频引擎发布命令。
pub async fn publish_command(
    state: &Arc<AppState>,
    command: &crate::models::AudioCommand,
) -> Result<(), AppError> {
    let json = serde_json::to_string(command)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Serialize error: {}", e)))?;

    let channel = &state.config.redis.command_channel;

    redis::cmd("PUBLISH")
        .arg(channel)
        .arg(&json)
        .query_async(&mut state.redis_conn.clone())
        .await
        .map_err(|e| AppError::Redis(e))?;

    tracing::info!("Published command to '{}': {}", channel, json);
    Ok(())
}
