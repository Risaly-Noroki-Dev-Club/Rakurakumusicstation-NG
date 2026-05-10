/// 电台后端 HTTP API 的路由模块。

pub mod auth;
pub mod songs;
pub mod playlist;
pub mod queue;
pub mod admin;
pub mod favorites;
pub mod ncm;

use crate::auth as crate_auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{DeviceUser, LyricsLineDto, SearchQuery, SongSummary};
use crate::templates::{
    AdminLogItem, AdminStats, AdminTemplate, ErrorTemplate, LibraryTemplate,
    NowPlayingTemplate, QueueHistoryItem, QueueTemplate, SettingsTemplate,
};
use askama_axum::IntoResponse;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Redirect,
    routing::get,
};
use sqlx::Row;
use std::sync::Arc;

/// 构建组合的应用程序路由器。
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // 页面路由（服务端渲染）
        .route("/", get(now_playing_page))
        .route("/library", get(library_page))
        .route("/queue", get(queue_page))
        .route("/settings", get(settings_page))
        .route("/admin", get(admin_redirect))
        .route("/admin/:tab", get(admin_page))
        // WebSocket 端点
        .route("/ws", get(crate::websocket::ws_handler))
        // 音频流端点
        .route("/stream", get(stream_handler))
        // 设备认证路由
        .nest("/api/auth", auth::auth_routes())
        // 歌曲库
        .nest("/api/songs", songs::song_routes())
        // 用户播放列表
        .nest("/api/playlists", playlist::playlist_routes())
        // 共享电台队列
        .nest("/api/queue", queue::queue_routes())
        // 管理端点
        .nest("/api/admin", admin::admin_routes())
        // 设备个人网易云账号
        .nest("/api/ncm", ncm::ncm_routes())
        // 收藏夹
        .nest("/api/favorites", favorites::favorites_routes())
        // 电台信息（公开）
        .route("/api/station", get(station_info))
        // 正在播放（公开）
        .route("/api/now-playing", get(queue::now_playing))
        .with_state(state)
}

async fn page_ctx(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(crate_auth::AuthUser, String, String, String, String, String, bool, bool), AppError> {
    let user = crate_auth::require_device_auth(headers, &state.db).await?;
    let is_admin = user.role == "admin";
    // 把 RwLockReadGuard 借的字段 clone 出来后立刻 drop guard，
    // 否则 std::sync::RwLockReadGuard 跨后面的 .await 会让整个 future 失去 Send。
    let (station_name, primary_color, bg_color) = {
        let station = state.station.read().unwrap_or_else(|e| e.into_inner());
        (
            station.name.clone(),
            station.primary_color.clone(),
            station.bg_color.clone(),
        )
    };
    let stream_url = state
        .config
        .audio_engine
        .resolve_stream_url(Some(headers), state.config.server.port);
    let ws_host = if state.config.server.host == "0.0.0.0" {
        "localhost".to_string()
    } else {
        state.config.server.host.clone()
    };
    let ws_url = format!("ws://{}:{}/ws", ws_host, state.config.server.port);
    let has_admin = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM device_users WHERE role = 'admin'"
    )
    .fetch_one(&state.db)
    .await
    .map(|r| r.0 > 0)
    .unwrap_or(false);
    Ok((
        user,
        station_name,
        primary_color,
        bg_color,
        stream_url,
        ws_url,
        is_admin,
        !has_admin,
    ))
}

async fn now_playing_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let (user, station_name, primary_color, bg_color, stream_url, ws_url, is_admin, _) =
        page_ctx(&state, &headers).await?;

    let playing = sqlx::query_as::<_, crate::models::QueueItem>(
        "SELECT * FROM queue_items WHERE status = 'playing' ORDER BY position ASC LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await?;

    let (song, duration_ms, lyrics_lines, cover_url) = if let Some(item) = playing {
        let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
            .bind(item.song_id)
            .fetch_optional(&state.db)
            .await?;
        let mut duration_ms = 0i64;
        let mut lyrics_lines = Vec::new();
        let mut cover_url = String::new();
        if let Some(ref s) = song {
            duration_ms = s.duration_ms;
            cover_url = format!("/api/songs/{}/cover", s.id);
            if !s.lyrics_path.is_empty() {
                let lrc_full = std::path::Path::new(&state.config.audio_engine.media_path)
                    .join(&s.lyrics_path);
                if let Ok(text) = std::fs::read_to_string(&lrc_full) {
                    let parsed = crate::lyrics::Lyrics::parse(&text);
                    lyrics_lines = parsed
                        .lines
                        .into_iter()
                        .map(|l| LyricsLineDto {
                            time_ms: l.time_ms,
                            text: l.text,
                        })
                        .collect();
                }
            }
        }
        (song.map(SongSummary::from), duration_ms, lyrics_lines, cover_url)
    } else {
        (None, 0, Vec::new(), String::new())
    };

    Ok(NowPlayingTemplate {
        title: "正在播放".into(),
        user,
        is_admin,
        station_name,
        primary_color,
        bg_color,
        page: "now-playing".into(),
        stream_url,
        ws_url,
        song,
        cover_url,
        duration_ms,
        lyrics_lines,
    })
}

async fn library_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    let (user, station_name, primary_color, bg_color, stream_url, ws_url, is_admin, _) =
        page_ctx(&state, &headers).await?;

    let query_str = query.q.unwrap_or_default();
    let songs = if !query_str.is_empty() {
        let pattern = format!("%{}%", query_str);
        sqlx::query_as::<_, crate::models::Song>(
            "SELECT * FROM songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ? ORDER BY created_at DESC LIMIT 50",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(SongSummary::from)
        .collect()
    } else {
        Vec::new()
    };

    let rows = sqlx::query(
        "SELECT p.id, p.device_user_id, p.name, p.is_public, p.created_at, COUNT(ps.song_id) as song_count
         FROM playlists p
         LEFT JOIN playlist_songs ps ON ps.playlist_id = p.id
         WHERE p.device_user_id = ?
         GROUP BY p.id
         ORDER BY p.created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let mut playlists = Vec::new();
    for r in rows {
        playlists.push(crate::models::PlaylistWithCount {
            id: r.get("id"),
            device_user_id: r.get("device_user_id"),
            name: r.get("name"),
            is_public: r.get("is_public"),
            song_count: r.get::<i64, _>("song_count"),
            created_at: chrono::NaiveDateTime::parse_from_str(
                &r.get::<String, _>("created_at"),
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap_or_default(),
        });
    }

    let ncm_status = None;

    Ok(LibraryTemplate {
        title: "曲库".into(),
        user,
        is_admin,
        station_name,
        primary_color,
        bg_color,
        page: "library".into(),
        stream_url,
        ws_url,
        query: query_str,
        songs,
        playlists,
        ncm_status,
    })
}

async fn queue_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let (user, station_name, primary_color, bg_color, stream_url, ws_url, is_admin, _) =
        page_ctx(&state, &headers).await?;

    let queue = crate::queue_manager::get_queue_display(&state.db).await?;
    let history = crate::queue_manager::get_history(&state.db, 20).await?;
    let history_items: Vec<QueueHistoryItem> = history
        .into_iter()
        .map(|h| {
            let song_obj = h.get("song").and_then(|v| v.as_object());
            QueueHistoryItem {
                id: h.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                song: song_obj.and_then(|s| {
                    Some(SongSummary {
                        id: s.get("id")?.as_i64()?,
                        title: s.get("title")?.as_str()?.to_string(),
                        artist: s.get("artist")?.as_str()?.to_string(),
                        album: s.get("album")?.as_str()?.to_string(),
                        duration_ms: s.get("duration_ms")?.as_i64()?,
                        has_lyrics: s.get("has_lyrics")?.as_bool()?,
                        has_cover: s.get("has_cover")?.as_bool()?,
                    })
                }),
                requested_by: h
                    .get("requested_by")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                played_at: h
                    .get("played_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            }
        })
        .collect();

    Ok(QueueTemplate {
        title: "队列".into(),
        user,
        is_admin,
        station_name,
        primary_color,
        bg_color,
        page: "queue".into(),
        stream_url,
        ws_url,
        queue,
        history: history_items,
    })
}

async fn settings_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let (user, station_name, primary_color, bg_color, stream_url, ws_url, is_admin, needs_setup) =
        page_ctx(&state, &headers).await?;

    Ok(SettingsTemplate {
        title: "设置".into(),
        user,
        is_admin,
        station_name,
        primary_color,
        bg_color,
        page: "settings".into(),
        stream_url,
        ws_url,
        needs_setup,
    })
}

async fn admin_redirect() -> Redirect {
    Redirect::to("/admin/stats")
}

async fn admin_page(
    State(state): State<Arc<AppState>>,
    Path(tab): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let user = crate_auth::require_admin_from_headers(&headers, &state.db).await?;
    let is_admin = true;
    // 跨 await 持 RwLockReadGuard 会让 future 失去 Send；先 clone 完字段再放手。
    let (station_name, primary_color, bg_color, settings) = {
        let station = state.station.read().unwrap_or_else(|e| e.into_inner());
        let settings = crate::models::SettingsResponse {
            station_name: station.name.clone(),
            subtitle: station.subtitle.clone(),
            primary_color: station.primary_color.clone(),
            secondary_color: station.secondary_color.clone(),
            bg_color: station.bg_color.clone(),
        };
        (
            station.name.clone(),
            station.primary_color.clone(),
            station.bg_color.clone(),
            settings,
        )
    };
    let stream_url = state
        .config
        .audio_engine
        .resolve_stream_url(Some(&headers), state.config.server.port);
    let ws_host = if state.config.server.host == "0.0.0.0" {
        "localhost".to_string()
    } else {
        state.config.server.host.clone()
    };
    let ws_url = format!("ws://{}:{}/ws", ws_host, state.config.server.port);

    let mut users = Vec::new();
    let mut songs = Vec::new();
    let mut stats = AdminStats::default();
    let mut logs = Vec::new();
    let ncm_status = None;

    match tab.as_str() {
        "users" => {
            users = sqlx::query_as::<_, DeviceUser>(
                "SELECT * FROM device_users ORDER BY created_at DESC",
            )
            .fetch_all(&state.db)
            .await?;
            let raw_logs = sqlx::query_as::<_, crate::models::AdminLog>(
                "SELECT * FROM admin_log ORDER BY created_at DESC LIMIT 100",
            )
            .fetch_all(&state.db)
            .await?;
            logs = raw_logs
                .into_iter()
                .map(|l| AdminLogItem {
                    created_at: l.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    action: l.action,
                    details: l.details,
                })
                .collect();
        }
        "songs" => {
            songs = sqlx::query_as::<_, crate::models::Song>(
                "SELECT * FROM songs ORDER BY created_at DESC LIMIT 200",
            )
            .fetch_all(&state.db)
            .await?
            .into_iter()
            .map(SongSummary::from)
            .collect();
        }
        "stats" => {
            let user_count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM device_users")
                    .fetch_one(&state.db)
                    .await?;
            let song_count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM songs")
                    .fetch_one(&state.db)
                    .await?;
            let queue_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM queue_items WHERE status IN ('pending', 'playing')",
            )
            .fetch_one(&state.db)
            .await?;
            let playlist_count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM playlists")
                    .fetch_one(&state.db)
                    .await?;
            stats = AdminStats {
                users: user_count.0,
                songs: song_count.0,
                queue_size: queue_count.0,
                playlists: playlist_count.0,
            };
        }
        "settings" | "upload" | "download" | "ncm" => {}
        _ => {}
    }

    Ok(AdminTemplate {
        title: "管理后台".into(),
        user,
        is_admin,
        station_name,
        primary_color,
        bg_color,
        page: "admin".into(),
        stream_url,
        ws_url,
        tab,
        users,
        songs,
        stats,
        logs,
        settings,
        ncm_status,
        download_running: false,
        download_log: String::new(),
    })
}

/// GET /stream — 音频流端点，从环形缓冲区广播音频数据
async fn stream_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Response {
    use radio_engine::config::AUDIO_CHUNK_SIZE;
    use std::time::{Duration, Instant};

    const SEND_TIMEOUT: Duration = Duration::from_secs(5);
    const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
    const WAIT_DATA_MS: u64 = 500;

    let (tx, response) = radio_engine::stream::create_stream_response();
    let buffer = state.ring_buffer.clone();

    tokio::spawn(async move {
        let reader = buffer.create_reader();
        let mut buf = vec![0u8; AUDIO_CHUNK_SIZE];
        let mut last_progress = Instant::now();

        loop {
            if tx.is_closed() {
                break;
            }
            if last_progress.elapsed() > IDLE_TIMEOUT {
                tracing::debug!("Stream idle timeout, closing");
                break;
            }

            let available = reader.wait_for_data(WAIT_DATA_MS).await;
            if available == 0 {
                continue;
            }

            let to_read = std::cmp::min(buf.len(), available);
            let n = reader.read(&mut buf[..to_read]);
            if n == 0 {
                continue;
            }

            let chunk = bytes::Bytes::copy_from_slice(&buf[..n]);
            match tokio::time::timeout(SEND_TIMEOUT, tx.send(chunk)).await {
                Ok(Ok(())) => {
                    last_progress = Instant::now();
                }
                Ok(Err(_)) => break,
                Err(_) => {
                    tracing::debug!("Stream send timeout — client likely dead");
                    break;
                }
            }
        }

        tracing::debug!("Stream client disconnected, reader cleaned up");
    });

    response
}

/// GET /api/station — 公开的电台信息
async fn station_info(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> axum::Json<serde_json::Value> {
    let ws_host = if state.config.server.host == "0.0.0.0" {
        "localhost"
    } else {
        &state.config.server.host
    };

    let has_admin = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM device_users WHERE role = 'admin'"
    )
    .fetch_one(&state.db)
    .await
    .map(|r| r.0 > 0)
    .unwrap_or(false);

    let station = state.station.read().unwrap_or_else(|e| e.into_inner());

    axum::Json(serde_json::json!({
        "name": station.name,
        "subtitle": station.subtitle,
        "primary_color": station.primary_color,
        "secondary_color": station.secondary_color,
        "bg_color": station.bg_color,
        "stream_url": state.config.audio_engine.resolve_stream_url(Some(&headers), state.config.server.port),
        "ws_url": format!("ws://{}:{}/ws", ws_host, state.config.server.port),
        "needs_setup": !has_admin,
    }))
}
