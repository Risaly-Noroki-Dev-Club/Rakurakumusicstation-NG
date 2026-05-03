/// 反映 SQL 模式的数据库模型。
/// SQLx 派生 FromRow 用于查询结果映射。

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ─── 用户 ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub banned_until: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub fn is_banned(&self) -> bool {
        match self.banned_until {
            Some(until) => until > chrono::Utc::now().naive_utc(),
            None => false,
        }
    }
}

/// 面向公众的用户信息（不含密码哈希）。
#[derive(Debug, Serialize)]
pub struct UserPublic {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub is_banned: bool,
    pub created_at: NaiveDateTime,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        let is_banned = u.is_banned();
        Self {
            id: u.id,
            username: u.username,
            role: u.role,
            is_banned,
            created_at: u.created_at,
        }
    }
}

// ─── 歌曲 ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Song {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: i32,
    pub duration_ms: i64,
    pub file_path: String,
    pub lyrics_path: String,
    pub cover_path: String,
    pub filesize: i64,
    pub created_at: NaiveDateTime,
}

/// 歌曲摘要，用于列表响应（出于安全考虑省略路径）。
#[derive(Debug, Serialize)]
pub struct SongSummary {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
    pub has_lyrics: bool,
    pub has_cover: bool,
}

impl From<Song> for SongSummary {
    fn from(s: Song) -> Self {
        Self {
            id: s.id,
            title: s.title,
            artist: s.artist,
            album: s.album,
            duration_ms: s.duration_ms,
            has_lyrics: !s.lyrics_path.is_empty(),
            has_cover: !s.cover_path.is_empty(),
        }
    }
}

// ─── 播放列表 ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Playlist {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub is_public: bool,
    pub created_at: NaiveDateTime,
}

/// 包含歌曲数量的播放列表。
#[derive(Debug, Serialize)]
pub struct PlaylistWithCount {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub is_public: bool,
    pub song_count: i64,
    pub created_at: NaiveDateTime,
}

// ─── 播放列表歌曲 ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaylistSong {
    pub id: i64,
    pub playlist_id: i64,
    pub song_id: i64,
    pub position: i32,
    pub added_at: NaiveDateTime,
}

// ─── 队列条目 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QueueItem {
    pub id: i64,
    pub song_id: i64,
    pub user_id: i64,
    pub status: String,       // 等待中 | 播放中 | 已播放 | 已跳过
    pub position: i32,
    pub added_at: NaiveDateTime,
    pub played_at: Option<NaiveDateTime>,
}

/// 为 API 响应丰富了歌曲和用户信息的队列条目。
#[derive(Debug, Serialize)]
pub struct QueueItemDisplay {
    pub id: i64,
    pub song: Option<SongSummary>,
    pub requested_by: String,
    pub status: String,
    pub position: i32,
    pub added_at: NaiveDateTime,
}

// ─── 播放历史 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlayHistory {
    pub id: i64,
    pub song_id: i64,
    pub user_id: Option<i64>,
    pub played_at: NaiveDateTime,
}

// ─── 管理日志 ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AdminLog {
    pub id: i64,
    pub admin_id: i64,
    pub action: String,
    pub details: String,
    pub created_at: NaiveDateTime,
}

// ─── 收藏 ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Favorite {
    pub id: i64,
    pub user_id: i64,
    pub song_id: Option<i64>,
    pub playlist_id: Option<i64>,
    pub created_at: NaiveDateTime,
}

// ─── JWT Claims ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject = 用户 ID，以字符串表示
    pub sub: String,
    /// 用于显示的用户名
    pub username: String,
    /// 用户角色
    pub role: String,
    /// 过期时间戳（自纪元以来的 UTC 秒数）
    pub exp: usize,
    /// 签发时间戳
    pub iat: usize,
}

// ─── 请求 / 响应 DTO ────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserPublic,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlaylistRequest {
    pub name: String,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Debug, Deserialize)]
pub struct AddSongToPlaylistRequest {
    pub song_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct AddToQueueRequest {
    pub song_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct MoveQueueItemRequest {
    pub new_position: i32,
}

#[derive(Debug, Deserialize)]
pub struct AdminActionRequest {
    pub user_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct SetRoleRequest {
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct NowPlaying {
    pub song: Option<SongSummary>,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub lyrics_line: Option<usize>,
    pub lyrics_text: Option<String>,
    pub started_at: Option<String>,
    pub stream_url: String,
    pub file_url: Option<String>,
}

/// 从 C++ 引擎通过 Redis 发送的播放状态消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub song_id: i64,
    pub file_path: String,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub lyrics_line: Option<usize>,
    pub status: String,       // 播放中 | 已停止 | 已暂停
    pub total_bytes_sent: u64,
    pub bitrate_kbps: u32,
    pub track_start_timestamp_ms: i64,
}

/// 通过 Redis 发送给 C++ 音频引擎的命令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCommand {
    /// 命令类型：跳过、播放、停止、音量
    #[serde(rename = "type")]
    pub cmd_type: String,
    pub song_id: Option<i64>,
    pub file_path: Option<String>,
}

/// 从 Rust 发送的队列事件，用于通知 C++ 引擎队列变更。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEvent {
    #[serde(rename = "type")]
    pub event_type: String,   // 下一首、跳过、清空
    pub song_id: Option<i64>,
    pub file_path: Option<String>,
}

/// 发送给已连接浏览器的 WebSocket 消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// 播放状态更新
    #[serde(rename = "playback_state")]
    PlaybackState {
        song_id: i64,
        title: String,
        artist: String,
        position_ms: i64,
        duration_ms: i64,
        lyrics_line: Option<usize>,
        lyrics_text: Option<String>,
        status: String,
        stream_url: String,
        file_url: Option<String>,
    },
    /// 队列变更通知
    #[serde(rename = "queue_update")]
    QueueUpdate {
        action: String,
        song_title: Option<String>,
        requested_by: Option<String>,
        queue_size: usize,
    },
    /// 服务器通知（例如"歌曲已开始""曲目已跳过"）
    #[serde(rename = "notice")]
    Notice {
        message: String,
        level: String,  // 信息、警告、错误
    },
    /// 心跳 ping - 客户端应回复 pong
    #[serde(rename = "ping")]
    Ping { timestamp: i64 },
}

/// 搜索查询参数
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 分页响应包装器
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// 通用 API 响应
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(msg.into()) }
    }
}

/// 用于无数据体的响应
pub type SimpleResponse = ApiResponse<serde_json::Value>;

// ─── 设置 DTO ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SaveSettingsRequest {
    pub station_name: Option<String>,
    pub subtitle: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub bg_color: Option<String>,
    pub admin_password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub station_name: String,
    pub subtitle: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub bg_color: String,
}

// ─── 下载状态 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub playlist: String,
    pub quality: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatus {
    pub running: bool,
    pub log: String,
}
