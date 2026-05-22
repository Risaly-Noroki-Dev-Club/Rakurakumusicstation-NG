use serde::{Deserialize, Serialize};

/// 歌词行 DTO（用于 WebSocket 序列化）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsLineDto {
    pub time_ms: i64,
    pub text: String,
}

/// 播放状态枚举（从引擎 re-export）。
pub use radio_engine::types::PlaybackStatus;

/// 发送给已连接浏览器的 WebSocket 消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "playback_state")]
    PlaybackState {
        song_id: i64,
        title: String,
        artist: String,
        position_ms: i64,
        duration_ms: i64,
        lyrics_line: Option<usize>,
        lyrics_lines: Option<Vec<LyricsLineDto>>,
        status: PlaybackStatus,
        stream_url: String,
        file_url: Option<String>,
        cover_url: Option<String>,
        timestamp_ms: i64,
    },
    #[serde(rename = "queue_update")]
    QueueUpdate {
        action: String,
        song_title: Option<String>,
        requested_by: Option<String>,
        queue_size: usize,
    },
    #[serde(rename = "notice")]
    Notice {
        message: String,
        level: String, // info | warning | error
    },
    #[serde(rename = "ping")]
    Ping { timestamp: i64 },
}
