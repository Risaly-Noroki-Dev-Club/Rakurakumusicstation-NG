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
    command: &radio_engine::types::AudioCommand,
) -> Result<(), AppError> {
    state.player_handle.send_command(command.clone());
    Ok(())
}

/// 启动引擎状态轮询器，将播放状态转发给 WebSocket 客户端。
/// 直接读取内嵌引擎的状态（不通过 HTTP）。
pub fn start_engine_state_poller(state: Arc<AppState>) {
    let state_clone = state.clone();

    tokio::spawn(async move {
        #[derive(Clone)]
        struct CachedSong {
            db_song_id: i64,
            title: String,
            artist: String,
            lyrics_lines: Option<Vec<crate::models::LyricsLineDto>>,
        }

        let mut last_index: i64 = 0;
        let mut cached: Option<CachedSong> = None;

        tracing::info!("Engine state poller started");

        loop {
            let ps = state_clone.player_handle.get_state();

            if ps.playlist_index != last_index && ps.playlist_index > 0 {
                last_index = ps.playlist_index;
                cached = None;

                if !ps.file_path.is_empty() {
                    let song_row = sqlx::query_as::<_, (i64, String, String, Option<String>)>(
                        "SELECT id, title, artist, lyrics_path FROM songs WHERE file_path = ?"
                    )
                    .bind(&ps.file_path)
                    .fetch_optional(&state_clone.db)
                    .await
                    .ok()
                    .flatten();

                    if let Some((db_song_id, title, artist, lyrics_path)) = song_row {
                        if let Err(e) = queue_manager::mark_playing(&state_clone.db, db_song_id).await {
                            tracing::error!("mark_playing failed for song {}: {}", db_song_id, e);
                        }

                        let lyrics_lines = lyrics_path.and_then(|path| {
                            if path.is_empty() {
                                return None;
                            }
                            let lrc_full = std::path::Path::new(
                                &state_clone.config.audio_engine.media_path,
                            )
                            .join(&path);
                            std::fs::read_to_string(&lrc_full).ok().map(|content| {
                                let parsed = crate::lyrics::Lyrics::parse(&content);
                                parsed.lines.into_iter().map(|l| crate::models::LyricsLineDto {
                                    time_ms: l.time_ms,
                                    text: l.text,
                                }).collect::<Vec<_>>()
                            })
                        });

                        cached = Some(CachedSong {
                            db_song_id,
                            title,
                            artist,
                            lyrics_lines,
                        });
                    }
                }
            }

            let (song_id, title, artist, lyrics_lines) = match cached.as_ref() {
                Some(c) => (c.db_song_id, c.title.clone(), c.artist.clone(), c.lyrics_lines.clone()),
                None => (ps.playlist_index, String::new(), String::new(), None),
            };

            let lyrics_line = lyrics_lines
                .as_ref()
                .and_then(|lines| {
                    lines.iter().enumerate().rev().find(|(_, l)| l.time_ms <= ps.position_ms)
                        .map(|(idx, _)| idx)
                });

            let enriched = WsMessage::PlaybackState {
                song_id,
                title,
                artist,
                position_ms: ps.position_ms,
                duration_ms: ps.duration_ms,
                lyrics_line,
                lyrics_lines,
                status: ps.status.clone(),
                stream_url: state_clone.config.audio_engine.resolve_stream_url(
                    None, state_clone.config.server.port
                ),
                file_url: if song_id > 0 {
                    Some(state_clone.config.audio_engine.resolve_file_url(song_id))
                } else {
                    None
                },
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
            };

            let _ = state_clone.ws_tx.send(
                serde_json::to_string(&enriched).unwrap_or_default()
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });
}
