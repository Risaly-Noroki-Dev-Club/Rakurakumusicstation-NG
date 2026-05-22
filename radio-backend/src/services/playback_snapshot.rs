//! Converts engine playback state into enriched WebSocket playback messages.

use crate::app::state::AppState;
use crate::models::{LyricsLineDto, WsMessage};
use crate::services::queue;

struct CachedSong {
    db_song_id: i64,
    title: String,
    artist: String,
    lyrics_lines: Option<Vec<LyricsLineDto>>,
}

pub(crate) struct PlaybackSnapshotCache {
    last_file_path: String,
    cached: Option<CachedSong>,
    /// 记录已向客户端发送过全量歌词的歌曲 ID，避免重复克隆。
    lyrics_broadcast_song_id: Option<i64>,
}

impl PlaybackSnapshotCache {
    pub(crate) fn new() -> Self {
        Self {
            last_file_path: String::new(),
            cached: None,
            lyrics_broadcast_song_id: None,
        }
    }

    pub(crate) async fn build_message(
        &mut self,
        state: &AppState,
        ps: &radio_engine::types::PlaybackState,
    ) -> WsMessage {
        self.refresh_on_song_change(state, ps).await;

        // 优先用 DB songs 里的 title/artist；查不到时回退到引擎自带的
        // 元数据（PlaybackState.title / artist），这样文件夹里手动塞的、
        // 或还没入库的歌也能正常显示，不会一直"等待播放"。
        let (song_id, title, artist) = match self.cached.as_ref() {
            Some(c) => (c.db_song_id, c.title.clone(), c.artist.clone()),
            None => {
                let id = ps.song_id.unwrap_or(-1);
                (id, ps.title.clone(), ps.artist.clone())
            }
        };

        let lyrics_lines_ref = self.cached.as_ref().and_then(|c| c.lyrics_lines.as_ref());
        let lyrics_line = lyrics_lines_ref.and_then(|lines| {
            lines
                .iter()
                .enumerate()
                .rev()
                .find(|(_, l)| l.time_ms <= ps.position_ms)
                .map(|(idx, _)| idx)
        });

        // 全量歌词只在歌曲切换后的首条消息中发送，后续只发行索引。
        let should_send_full_lyrics = self.lyrics_broadcast_song_id != Some(song_id);
        let lyrics_lines_payload = if should_send_full_lyrics {
            self.lyrics_broadcast_song_id = Some(song_id);
            lyrics_lines_ref.cloned()
        } else {
            None
        };

        WsMessage::PlaybackState {
            song_id,
            title,
            artist,
            position_ms: ps.position_ms,
            duration_ms: ps.duration_ms,
            lyrics_line,
            lyrics_lines: lyrics_lines_payload,
            status: ps.status.clone(),
            stream_url: state.config.audio_engine.resolve_stream_url(
                None,
                state.config.server.port,
                &state.config.server.base_path,
            ),
            file_url: if song_id > 0 {
                Some(
                    state
                        .config
                        .audio_engine
                        .resolve_file_url(song_id, &state.config.server.base_path),
                )
            } else {
                None
            },
            cover_url: if song_id > 0 {
                Some(
                    state
                        .config
                        .audio_engine
                        .resolve_cover_url(song_id, &state.config.server.base_path),
                )
            } else {
                None
            },
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    async fn refresh_on_song_change(
        &mut self,
        state: &AppState,
        ps: &radio_engine::types::PlaybackState,
    ) {
        // 切歌检测改用 file_path：playlist_index 对请求队列曲来说固定为 -1，
        // 连着两首请求曲不会换 index，但 file_path 一定不同。
        let song_changed = ps.file_path != self.last_file_path && !ps.file_path.is_empty();
        if !song_changed {
            return;
        }

        self.last_file_path = ps.file_path.clone();
        self.cached = None;
        self.lyrics_broadcast_song_id = None;

        let song_row = sqlx::query_as::<_, (i64, String, String, String, String)>(
            "SELECT id, title, artist, cover_path, lyrics_path FROM songs WHERE file_path = ?",
        )
        .bind(&ps.file_path)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        let Some((db_song_id, title, artist, _cover_path, lyrics_path)) = song_row else {
            return;
        };

        if let Err(e) = queue::mark_playing(&state.db, db_song_id).await {
            tracing::error!("mark_playing failed for song {}: {}", db_song_id, e);
        }

        self.cached = Some(CachedSong {
            db_song_id,
            title,
            artist,
            lyrics_lines: load_lyrics_lines(state, &lyrics_path),
        });
    }
}

fn load_lyrics_lines(state: &AppState, lyrics_path: &str) -> Option<Vec<LyricsLineDto>> {
    if lyrics_path.is_empty() {
        return None;
    }

    let lrc_full = std::path::Path::new(&state.config.audio_engine.media_path).join(lyrics_path);
    std::fs::read_to_string(&lrc_full).ok().map(|content| {
        let parsed = crate::lyrics::Lyrics::parse(&content);
        parsed
            .lines
            .into_iter()
            .map(|l| LyricsLineDto {
                time_ms: l.time_ms,
                text: l.text,
            })
            .collect::<Vec<_>>()
    })
}
