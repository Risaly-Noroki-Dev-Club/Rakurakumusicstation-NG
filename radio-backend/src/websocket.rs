/// WebSocket 处理：升级 HTTP 连接并向所有已连接客户端广播播放状态。
///
/// 架构：
/// - WebSocket 连接订阅 Tokio 广播频道，将消息转发给客户端。
/// - 一个后台任务 HTTP 轮询 C++ 引擎的 GET /state，解析后广播。
/// - 每 30 秒发送一次心跳 ping，检测断开的连接。

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

/// 启动后台任务，HTTP 轮询 C++ 引擎的 /state 端点并广播到 WebSocket。
pub async fn start_state_poller(state: Arc<AppState>) {
    let state_url = state.config.audio_engine.state_url();

    tokio::spawn(async move {
        let mut last_song_id: i64 = 0;
        let mut cached_lyrics: Option<crate::lyrics::Lyrics> = None;
        let mut cached_lrc_text: Option<String> = None;

        tracing::info!("State poller started, polling {}", state_url);

        loop {
            match state.http_client.get(&state_url).send().await {
                Ok(resp) => {
                    match resp.json::<PlaybackState>().await {
                        Ok(ps) => {
                            let enrich = enrich_playback_state(
                                &state, &ps,
                                &mut last_song_id,
                                &mut cached_lyrics,
                                &mut cached_lrc_text,
                            ).await;

                            let _ = state.ws_tx.send(enrich);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse state from engine: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to poll engine state at {}: {}", state_url, e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
}

/// 用 DB 数据丰富 C++ 引擎的原始播放状态消息。
async fn enrich_playback_state(
    state: &Arc<AppState>,
    ps: &PlaybackState,
    last_song_id: &mut i64,
    cached_lyrics: &mut Option<crate::lyrics::Lyrics>,
    cached_lrc_text: &mut Option<String>,
) -> String {
    if ps.song_id != *last_song_id && ps.song_id > 0 {
        *last_song_id = ps.song_id;

        if let Err(e) = queue_manager::mark_playing(&state.db, ps.song_id).await {
            tracing::error!("mark_playing failed for song {}: {}", ps.song_id, e);
        }

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

    let lyrics_line = cached_lyrics
        .as_ref()
        .and_then(|l| l.line_at(ps.position_ms));

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
        stream_url: state.config.audio_engine.resolve_stream_url(),
        file_url: if ps.song_id > 0 {
            Some(state.config.audio_engine.resolve_file_url(ps.song_id))
        } else {
            None
        },
    };

    serde_json::to_string(&enriched).unwrap_or_default()
}

/// 通过 HTTP POST 向 C++ 音频引擎发送命令。
pub async fn publish_command(
    state: &Arc<AppState>,
    command: &crate::models::AudioCommand,
) -> Result<(), AppError> {
    let url = state.config.audio_engine.command_url();

    let resp = state.http_client
        .post(&url)
        .json(command)
        .send()
        .await?;

    if resp.status().is_success() {
        tracing::info!("Published command to {}: {:?}", url, command);
        Ok(())
    } else {
        tracing::warn!("Engine command returned status {} for {}", resp.status(), url);
        Ok(())
    }
}
