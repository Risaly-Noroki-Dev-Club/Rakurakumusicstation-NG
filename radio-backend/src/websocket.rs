/// WebSocket 处理：升级 HTTP 连接并向所有已连接客户端广播消息。
///
/// 从内嵌音频引擎直接获取播放状态（不再通过 HTTP 轮询 C++ 引擎）。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::WsMessage;
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
    command: &crate::models::AudioCommand,
) -> Result<(), AppError> {
    let engine_cmd = radio_engine::types::AudioCommand {
        cmd_type: command.cmd_type.clone(),
        song_id: command.song_id,
        file_path: command.file_path.clone(),
    };
    state.player_handle.send_command(engine_cmd);
    Ok(())
}

/// 启动引擎状态轮询器，将播放状态转发给 WebSocket 客户端。
/// 直接读取内嵌引擎的状态（不通过 HTTP）。
pub fn start_engine_state_poller(state: Arc<AppState>) {
    let state_clone = state.clone();

    tokio::spawn(async move {
        let mut last_song_id: i64 = 0;
        let mut cached_lyrics: Option<crate::lyrics::Lyrics> = None;
        let mut cached_lrc_text: Option<String> = None;

        tracing::info!("Engine state poller started");

        loop {
            let ps = state_clone.player_handle.get_state();

            if ps.song_id != last_song_id && ps.song_id > 0 {
                last_song_id = ps.song_id;

                // Look up song in DB by file_path (engine uses file index as song_id)
                let song_id_from_db = if !ps.file_path.is_empty() {
                    sqlx::query_as::<_, (i64,)>(
                        "SELECT id FROM songs WHERE file_path = ?"
                    )
                    .bind(&ps.file_path)
                    .fetch_optional(&state_clone.db)
                    .await
                    .ok()
                    .flatten()
                    .map(|(id,)| id)
                } else {
                    None
                };

                if let Some(db_song_id) = song_id_from_db {
                    if let Err(e) = queue_manager::mark_playing(&state_clone.db, db_song_id).await {
                        tracing::error!("mark_playing failed for song {}: {}", db_song_id, e);
                    }

                    cached_lyrics = None;
                    cached_lrc_text = None;
                    if let Ok(Some(song)) = sqlx::query_as::<_, crate::models::Song>(
                        "SELECT * FROM songs WHERE id = ?"
                    )
                    .bind(db_song_id)
                    .fetch_optional(&state_clone.db)
                    .await
                    {
                        if !song.lyrics_path.is_empty() {
                            let lrc_full = std::path::Path::new(
                                &state_clone.config.audio_engine.media_path,
                            )
                            .join(&song.lyrics_path);
                            if let Ok(content) = std::fs::read_to_string(&lrc_full) {
                                cached_lyrics = Some(crate::lyrics::Lyrics::parse(&content));
                                cached_lrc_text = Some(content);
                            }
                        }
                    }
                }
            }

            let lyrics_line = cached_lyrics
                .as_ref()
                .and_then(|l| l.line_at(ps.position_ms));

            // Get title/artist from DB by matching file_path
            let (title, artist) = if !ps.file_path.is_empty() {
                sqlx::query_as::<_, (String, String)>(
                    "SELECT title, artist FROM songs WHERE file_path = ?"
                )
                .bind(&ps.file_path)
                .fetch_optional(&state_clone.db)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| (String::new(), String::new()))
            } else {
                (String::new(), String::new())
            };

            let db_song_id = if !ps.file_path.is_empty() {
                sqlx::query_as::<_, (i64,)>(
                    "SELECT id FROM songs WHERE file_path = ?"
                )
                .bind(&ps.file_path)
                .fetch_optional(&state_clone.db)
                .await
                .ok()
                .flatten()
                .map(|(id,)| id)
            } else {
                None
            };

            let enriched = WsMessage::PlaybackState {
                song_id: db_song_id.unwrap_or(ps.song_id),
                title,
                artist,
                position_ms: ps.position_ms,
                duration_ms: ps.duration_ms,
                lyrics_line,
                lyrics_text: cached_lrc_text.clone(),
                status: ps.status.clone(),
                stream_url: state_clone.config.audio_engine.resolve_stream_url(),
                file_url: db_song_id.map(|id| {
                    state_clone.config.audio_engine.resolve_file_url(id)
                }),
            };

            let _ = state_clone.ws_tx.send(
                serde_json::to_string(&enriched).unwrap_or_default()
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
}
