use crate::auth::AuthUser;
use crate::models::{
    DeviceUser, LyricsLineDto, NcmStatus, PlaylistWithCount, QueueItemDisplay, SongSummary,
};
use askama::Template;

#[derive(Template)]
#[template(path = "now_playing.html")]
pub struct NowPlayingTemplate {
    pub title: String,
    pub user: AuthUser,
    pub is_admin: bool,
    pub station_name: String,
    pub primary_color: String,
    pub bg_color: String,
    pub page: String,
    pub stream_url: String,
    pub ws_url: String,
    pub song: Option<SongSummary>,
    pub cover_url: String,
    pub duration_ms: i64,
    pub lyrics_lines: Vec<LyricsLineDto>,
}

#[derive(Template)]
#[template(path = "library.html")]
pub struct LibraryTemplate {
    pub title: String,
    pub user: AuthUser,
    pub is_admin: bool,
    pub station_name: String,
    pub primary_color: String,
    pub bg_color: String,
    pub page: String,
    pub stream_url: String,
    pub ws_url: String,
    pub query: String,
    pub songs: Vec<SongSummary>,
    pub playlists: Vec<PlaylistWithCount>,
    pub ncm_status: Option<NcmStatus>,
}

#[derive(Template)]
#[template(path = "queue.html")]
pub struct QueueTemplate {
    pub title: String,
    pub user: AuthUser,
    pub is_admin: bool,
    pub station_name: String,
    pub primary_color: String,
    pub bg_color: String,
    pub page: String,
    pub stream_url: String,
    pub ws_url: String,
    pub queue: Vec<QueueItemDisplay>,
    pub history: Vec<QueueHistoryItem>,
}

#[derive(Clone, Debug)]
pub struct QueueHistoryItem {
    pub id: i64,
    pub song: Option<SongSummary>,
    pub requested_by: String,
    pub played_at: String,
}

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    pub title: String,
    pub user: AuthUser,
    pub is_admin: bool,
    pub station_name: String,
    pub primary_color: String,
    pub bg_color: String,
    pub page: String,
    pub stream_url: String,
    pub ws_url: String,
    pub needs_setup: bool,
}

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {
    pub title: String,
    pub user: AuthUser,
    pub is_admin: bool,
    pub station_name: String,
    pub primary_color: String,
    pub bg_color: String,
    pub page: String,
    pub stream_url: String,
    pub ws_url: String,
    pub tab: String,
    pub users: Vec<DeviceUser>,
    pub songs: Vec<SongSummary>,
    pub stats: AdminStats,
    pub logs: Vec<AdminLogItem>,
    pub settings: crate::models::SettingsResponse,
    pub ncm_status: Option<NcmStatus>,
    pub download_running: bool,
    pub download_log: String,
}

#[derive(Clone, Debug, Default)]
pub struct AdminStats {
    pub users: i64,
    pub songs: i64,
    pub queue_size: i64,
    pub playlists: i64,
}

#[derive(Clone, Debug)]
pub struct AdminLogItem {
    pub created_at: String,
    pub action: String,
    pub details: String,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub title: String,
    pub user: Option<AuthUser>,
    pub is_admin: bool,
    pub station_name: String,
    pub primary_color: String,
    pub bg_color: String,
    pub page: String,
    pub stream_url: String,
    pub ws_url: String,
    pub message: String,
}
