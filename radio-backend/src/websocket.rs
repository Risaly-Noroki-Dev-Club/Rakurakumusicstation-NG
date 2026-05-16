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
///
/// ## 内存优化
/// - 仅在有订阅者时才发送消息
/// - 歌词仅在切换歌曲时解析一次并缓存，且只在首条消息中发送全量歌词
/// - 后续消息只发送当前歌词行索引，避免每 500ms 克隆数十 KB 的歌词数组
pub fn start_engine_state_poller(state: Arc<AppState>) {
    let state_clone = state.clone();

    tokio::spawn(async move {
        struct CachedSong {
            db_song_id: i64,
            title: String,
            artist: String,
            has_cover: bool,
            lyrics_lines: Option<Vec<crate::models::LyricsLineDto>>,
        }

        let mut last_file_path = String::new();
        let mut cached: Option<CachedSong> = None;
        // 记录已向客户端发送过全量歌词的歌曲 ID，避免重复克隆
        let mut lyrics_broadcast_song_id: Option<i64> = None;

        tracing::info!("Engine state poller started");

        loop {
            let ps = state_clone.player_handle.get_state();

            // 切歌检测改用 file_path：playlist_index 对请求队列曲来说固定为 -1，
            // 连着两首请求曲不会换 index，但 file_path 一定不同。
            let song_changed = ps.file_path != last_file_path && !ps.file_path.is_empty();

            if song_changed {
                last_file_path = ps.file_path.clone();
                cached = None;
                lyrics_broadcast_song_id = None;

                if !ps.file_path.is_empty() {
                    let song_row = sqlx::query_as::<_, (i64, String, String, String, String)>(
                        "SELECT id, title, artist, cover_path, lyrics_path FROM songs WHERE file_path = ?"
                    )
                    .bind(&ps.file_path)
                    .fetch_optional(&state_clone.db)
                    .await
                    .ok()
                    .flatten();

                    if let Some((db_song_id, title, artist, cover_path, lyrics_path)) = song_row {
                        if let Err(e) =
                            queue_manager::mark_playing(&state_clone.db, db_song_id).await
                        {
                            tracing::error!("mark_playing failed for song {}: {}", db_song_id, e);
                        }

                        let lyrics_lines = if lyrics_path.is_empty() {
                            None
                        } else {
                            let lrc_full =
                                std::path::Path::new(&state_clone.config.audio_engine.media_path)
                                    .join(&lyrics_path);
                            std::fs::read_to_string(&lrc_full).ok().map(|content| {
                                let parsed = crate::lyrics::Lyrics::parse(&content);
                                parsed
                                    .lines
                                    .into_iter()
                                    .map(|l| crate::models::LyricsLineDto {
                                        time_ms: l.time_ms,
                                        text: l.text,
                                    })
                                    .collect::<Vec<_>>()
                            })
                        };

                        cached = Some(CachedSong {
                            db_song_id,
                            title,
                            artist,
                            has_cover: !cover_path.is_empty(),
                            lyrics_lines,
                        });
                    }
                }
            }

            // 仅在有活跃订阅者时才发送消息
            if state_clone.ws_tx.receiver_count() > 0 {
                // 优先用 DB songs 里的 title/artist；查不到时回退到引擎自带的
                // 元数据（PlaybackState.title / artist），这样文件夹里手动塞的、
                // 或还没入库的歌也能正常显示，不会一直"等待播放"。
                let (song_id, title, artist) = match cached.as_ref() {
                    Some(c) => (c.db_song_id, c.title.clone(), c.artist.clone()),
                    None => {
                        let id = ps.song_id.unwrap_or(-1);
                        (id, ps.title.clone(), ps.artist.clone())
                    }
                };

                let lyrics_lines_ref = cached.as_ref().and_then(|c| c.lyrics_lines.as_ref());

                let lyrics_line = lyrics_lines_ref.and_then(|lines| {
                    lines
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, l)| l.time_ms <= ps.position_ms)
                        .map(|(idx, _)| idx)
                });

                // 全量歌词只在歌曲切换后的首条消息中发送，后续只发行索引
                let should_send_full_lyrics = lyrics_broadcast_song_id != Some(song_id);
                let lyrics_lines_payload = if should_send_full_lyrics {
                    lyrics_broadcast_song_id = Some(song_id);
                    lyrics_lines_ref.cloned()
                } else {
                    None
                };

                let enriched = WsMessage::PlaybackState {
                    song_id,
                    title,
                    artist,
                    position_ms: ps.position_ms,
                    duration_ms: ps.duration_ms,
                    lyrics_line,
                    lyrics_lines: lyrics_lines_payload,
                    status: ps.status.clone(),
                    stream_url: state_clone.config.audio_engine.resolve_stream_url(
                        None,
                        state_clone.config.server.port,
                        &state_clone.config.server.base_path,
                    ),
                    file_url: if song_id > 0 {
                        Some(
                            state_clone
                                .config
                                .audio_engine
                                .resolve_file_url(song_id, &state_clone.config.server.base_path),
                        )
                    } else {
                        None
                    },
                    cover_url: if song_id > 0
                        && cached.as_ref().map(|c| c.has_cover).unwrap_or(false)
                    {
                        Some(
                            state_clone
                                .config
                                .audio_engine
                                .resolve_cover_url(song_id, &state_clone.config.server.base_path),
                        )
                    } else {
                        None
                    },
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                };

                let _ = state_clone
                    .ws_tx
                    .send(serde_json::to_string(&enriched).unwrap_or_default());
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
}
