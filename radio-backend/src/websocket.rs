/// WebSocket 处理：升级 HTTP 连接并向所有已连接客户端广播播放状态。
///
/// 架构：
/// - 在 WebSocket 连接时，处理器订阅 Tokio 广播频道并将消息
///   转发给客户端。
/// - 一个后台任务订阅 Redis `playback_state` 频道，并将消息
///   重新发布到 Tokio 广播，供所有 WebSocket 客户端接收。
/// - 每 30 秒发送一次心跳 ping，以检测断开的连接。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::{PlaybackState, WsMessage};
use crate::queue_manager;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
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
            // 接收广播消息
            msg = rx.recv() => {
                match msg {
                    Ok(msg) => {
                        if sender.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged by {} messages", n);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // 接收客户端消息
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
            // 心跳
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
    // 仅在无接收者时 Send 返回 Err，这是可接受的。
    let _ = state.ws_tx.send(json);
}

/// 启动后台任务，订阅 Redis `playback_state` 并广播到 WebSocket 客户端。
/// 反序列化 C++ 引擎的 PlaybackState JSON，检测曲目切换触发
/// mark_playing，计算当前歌词行，查询 DB 补充 title/artist。
pub async fn start_redis_subscriber(state: Arc<AppState>) {
    let channel = state.config.redis.playback_channel.clone();
    let channel_for_log = channel.clone();

    tokio::spawn(async move {
        let mut last_song_id: i64 = 0;
        let mut cached_lyrics: Option<crate::lyrics::Lyrics> = None;
        let mut cached_lrc_text: Option<String> = None;

        loop {
            match redis::Client::open(state.config.redis.url.as_str()) {
                Ok(client) => {
                    #[allow(deprecated)]
                    match client.get_async_connection().await {
                        Ok(conn) => {
                            let mut pubsub = conn.into_pubsub();
                            if pubsub.subscribe(&channel).await.is_ok() {
                                tracing::info!("Subscribed to Redis channel: {}", channel);

                                loop {
                                    match pubsub.on_message().next().await {
                                        Some(msg) => {
                                            let payload: String = msg.get_payload().unwrap_or_default();

                                            // 尝试解析为 PlaybackState
                                            match serde_json::from_str::<PlaybackState>(&payload) {
                                                Ok(ps) => {
                                                    let enrich = enrich_playback_state(
                                                        &state, &ps,
                                                        &mut last_song_id,
                                                        &mut cached_lyrics,
                                                        &mut cached_lrc_text,
                                                    ).await;

                                                    let _ = state.ws_tx.send(enrich);
                                                }
                                                Err(_) => {
                                                    // 无法解析则原样转发
                                                    let _ = state.ws_tx.send(payload);
                                                }
                                            }
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

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    tracing::info!("Redis subscriber task started for channel: {}", channel_for_log);
}

/// 用 DB 数据丰富 C++ 引擎的原始播放状态消息：
/// - 检测曲目切换 → 调用 mark_playing
/// - 用当前 position_ms 计算歌词行 (lyrics_line)
/// - 查询歌曲的 title / artist / lyrics_text
/// 返回序列化为 JSON 的、可供前端直接使用的 WsMessage。
async fn enrich_playback_state(
    state: &Arc<AppState>,
    ps: &PlaybackState,
    last_song_id: &mut i64,
    cached_lyrics: &mut Option<crate::lyrics::Lyrics>,
    cached_lrc_text: &mut Option<String>,
) -> String {
    // 曲目切换
    if ps.song_id != *last_song_id && ps.song_id > 0 {
        *last_song_id = ps.song_id;

        if let Err(e) = queue_manager::mark_playing(&state.db, ps.song_id).await {
            tracing::error!("mark_playing failed for song {}: {}", ps.song_id, e);
        }

        // 加载新的 LRC
        *cached_lyrics = None;
        *cached_lrc_text = None;
        if let Ok(Some(song)) = sqlx::query_as::<_, crate::models::Song>(
            "SELECT * FROM songs WHERE id = ?"
        )
        .bind(ps.song_id)
        .fetch_optional(&state.db)
        .await
        {
            if !song.lyrics_path.is_empty() {
                let lrc_full = std::path::Path::new(&state.config.audio_engine.media_path)
                    .join(&song.lyrics_path);
                if let Ok(content) = std::fs::read_to_string(&lrc_full) {
                    *cached_lyrics = Some(crate::lyrics::Lyrics::parse(&content));
                    *cached_lrc_text = Some(content);
                }
            }
        }
    }

    // 计算当前歌词行
    let lyrics_line = cached_lyrics
        .as_ref()
        .and_then(|l| l.line_at(ps.position_ms));

    // 查询 DB 补充 title / artist
    let (title, artist) = if ps.song_id > 0 {
        sqlx::query_as::<_, (String, String)>(
            "SELECT title, artist FROM songs WHERE id = ?"
        )
        .bind(ps.song_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| (String::new(), String::new()))
    } else {
        (String::new(), String::new())
    };

    let enriched = WsMessage::PlaybackState {
        song_id: ps.song_id,
        title,
        artist,
        position_ms: ps.position_ms,
        duration_ms: ps.duration_ms,
        lyrics_line,
        lyrics_text: cached_lrc_text.clone(),
        status: ps.status.clone(),
        stream_url: format!("{}:{}/stream", state.config.audio_engine.base_url, 2240),
        file_url: if ps.song_id > 0 {
            Some(format!("{}:{}/file/{}", state.config.audio_engine.base_url, 2240, ps.song_id))
        } else {
            None
        },
    };

    serde_json::to_string(&enriched).unwrap_or_default()
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
        .query_async::<_, ()>(&mut state.redis_conn.clone())
        .await
        .map_err(|e| AppError::Redis(e))?;

    tracing::info!("Published command to '{}': {}", channel, json);
    Ok(())
}
