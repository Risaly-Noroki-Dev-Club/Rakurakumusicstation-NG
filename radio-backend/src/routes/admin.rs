/// 管理员专用路由：用户管理、系统统计、歌曲管理、设置、上传、下载。
///
/// 本模块包含以下端点：
/// - GET  /api/admin/users            列出所有用户
/// - POST /api/admin/users/{id}/ban   封禁用户
/// - POST /api/admin/users/{id}/unban 解封用户
/// - PUT  /api/admin/users/{id}/role  更改用户角色（提权/降权）
/// - GET  /api/admin/stats            系统统计
/// - GET  /api/admin/logs             管理日志
/// - POST /api/admin/rescan-songs     重新扫描媒体目录
/// - GET  /api/admin/settings         获取系统设置
/// - POST /api/admin/settings         保存系统设置
/// - POST /api/admin/upload           上传音乐文件
/// - DELETE /api/admin/songs/{id}     删除歌曲
/// - POST /api/admin/playlist/next    下一首（向 C++ 引擎发送 skip 命令）
/// - POST /api/admin/playlist/prev    上一首（通过 Redis queue_event）
/// - POST /api/admin/download         批量下载歌单
/// - GET  /api/admin/download/status  获取下载状态
/// - POST /api/admin/ncm              保存网易云账号设置
/// - GET  /api/admin/ncm              获取网易云账号状态
/// - POST /api/admin/ncm/test         测试网易云登录
/// - POST /api/admin/ncm/logout       退出登录（管理会话）
/// - GET  /api/admin/songs            获取所有歌曲（管理用完整列表）

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{
    ApiResponse, AudioCommand, DownloadRequest, DownloadStatus,
    SaveSettingsRequest, SetRoleRequest, SettingsResponse,
};
use crate::websocket;
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::{Arc, Mutex, OnceLock};

/// 全局下载状态，受 Mutex 保护
fn download_state() -> &'static Mutex<DownloadStatus> {
    static DL: OnceLock<Mutex<DownloadStatus>> = OnceLock::new();
    DL.get_or_init(|| Mutex::new(DownloadStatus {
        running: false,
        log: String::new(),
    }))
}

pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        // 用户管理
        .route("/users", get(list_users))
        .route("/users/:id/ban", post(ban_user))
        .route("/users/:id/unban", post(unban_user))
        .route("/users/:id/role", put(set_user_role))
        // 统计与日志
        .route("/stats", get(stats))
        .route("/logs", get(get_logs))
        // 歌曲管理
        .route("/rescan-songs", post(rescan_songs))
        .route("/songs", get(list_all_songs))
        .route("/songs/:id", delete(delete_song))
        // 上传 (带 100MB body limit)
        .nest("/upload", Router::new()
            .route("/", post(upload_song))
            .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        )
        // 系统设置
        .route("/settings", get(get_settings).post(save_settings))
        // 播放控制
        .route("/playlist/next", post(skip_next))
        .route("/playlist/prev", post(skip_prev))
        // 批量下载
        .route("/download", post(start_download))
        .route("/download/status", get(download_status))
        // 网易云账号
        .route("/ncm", get(get_ncm_settings).post(save_ncm_settings))
        .route("/ncm/test", post(test_ncm_login))
        // 管理登录（退出）
        .route("/logout", post(logout))
}

// ─── 身份验证辅助函数 ─────────────────────────────────────

/// 从请求头中提取已认证管理员用户。
pub async fn get_admin(state: &AppState, headers: &HeaderMap) -> Result<crate::auth::AuthUser, AppError> {
    auth::require_admin_from_headers(headers, &state.db, &state.jwt_secret).await
}

// ─── 用户管理 ─────────────────────────────────────────────

/// GET /api/admin/users — 列出所有用户
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<crate::models::UserPublic>>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let users = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;

    let public: Vec<crate::models::UserPublic> = users.into_iter().map(|u| u.into()).collect();

    Ok(Json(ApiResponse::ok(public)))
}

/// POST /api/admin/users/{id}/ban — 封禁用户（禁用队列提交）
pub async fn ban_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    // 防止封禁自己
    if user_id == admin.id {
        return Err(AppError::BadRequest("Cannot ban yourself".into()));
    }

    // 将 banned_until 设置为遥远的未来（100 年）
    sqlx::query("UPDATE users SET banned_until = datetime('now', '+100 years') WHERE id = ?")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    // 记录
    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'ban_user', ?)")
        .bind(admin.id)
        .bind(format!("Banned user {}", user_id))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("User banned".into())))
}

/// POST /api/admin/users/{id}/unban
pub async fn unban_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    sqlx::query("UPDATE users SET banned_until = NULL WHERE id = ?")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'unban_user', ?)")
        .bind(admin.id)
        .bind(format!("Unbanned user {}", user_id))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("User unbanned".into())))
}

/// PUT /api/admin/users/{id}/role — 更改用户角色（提权/降权）
pub async fn set_user_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<SetRoleRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    if body.role != "admin" && body.role != "user" {
        return Err(AppError::BadRequest("Role must be 'admin' or 'user'".into()));
    }

    if user_id == admin.id {
        return Err(AppError::BadRequest("Cannot change your own role".into()));
    }

    let target = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    let old_role = target.role.clone();
    if old_role == body.role {
        return Ok(Json(ApiResponse::ok(format!("User '{}' already has role '{}'", target.username, body.role))));
    }

    sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(&body.role)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    let action = if body.role == "admin" { "promote_user" } else { "demote_user" };
    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, ?, ?)")
        .bind(admin.id)
        .bind(action)
        .bind(format!("Changed user '{}' ({}) role from '{}' to '{}'", target.username, user_id, old_role, body.role))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok(format!("User '{}' role changed to '{}'", target.username, body.role))))
}

// ─── 统计 ────────────────────────────────────────────────

/// GET /api/admin/stats — 系统统计
pub async fn stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    let song_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM songs")
        .fetch_one(&state.db)
        .await?;

    let queue_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM queue_items WHERE status IN ('pending', 'playing')"
    )
    .fetch_one(&state.db)
    .await?;

    let playlist_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM playlists")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "users": user_count.0,
        "songs": song_count.0,
        "queue_size": queue_count.0,
        "playlists": playlist_count.0,
    }))))
}

// ─── 日志 ────────────────────────────────────────────────

/// GET /api/admin/logs — 获取最近的管理日志
pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let logs = sqlx::query_as::<_, crate::models::AdminLog>(
        "SELECT * FROM admin_log ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&state.db)
    .await?;

    let result: Vec<serde_json::Value> = logs
        .into_iter()
        .map(|log| {
            serde_json::json!({
                "id": log.id,
                "admin_id": log.admin_id,
                "action": log.action,
                "details": log.details,
                "created_at": log.created_at,
            })
        })
        .collect();

    Ok(Json(ApiResponse::ok(result)))
}

// ─── 重新扫描歌曲 ────────────────────────────────────────

/// 查找音频文件旁的封面图片
pub(crate) fn find_cover(audio_path: &std::path::Path, media_root: &std::path::Path) -> String {
    let cover_names = ["cover.jpg", "cover.png", "cover.jpeg",
                       "folder.jpg", "folder.png",
                       "album.jpg", "album.png",
                       "front.jpg", "front.png",
                       "AlbumCover.jpg", "AlbumCover.png"];
    let parent = audio_path.parent().unwrap_or_else(|| std::path::Path::new("."));

    for name in &cover_names {
        let candidate = parent.join(name);
        if candidate.exists() {
            return candidate.strip_prefix(media_root)
                .unwrap_or(&candidate)
                .to_string_lossy()
                .to_string();
        }
    }

    if let Some(stem) = audio_path.file_stem() {
        for ext in &["jpg", "png", "jpeg"] {
            let candidate = parent.join(format!("{}.{}", stem.to_string_lossy(), ext));
            if candidate.exists() {
                return candidate.strip_prefix(media_root)
                    .unwrap_or(&candidate)
                    .to_string_lossy()
                    .to_string();
            }
        }
    }

    String::new()
}

/// 通过 ffprobe 获取音频时长（fork+exec）。
pub(crate) fn get_duration(path: &std::path::Path) -> Option<i64> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).ok()?;
        let duration_secs: f64 = stdout.trim().parse().ok()?;
        Some((duration_secs * 1000.0) as i64)
    } else {
        None
    }
}

pub async fn rescan_songs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    let media_path = std::path::Path::new(&state.config.audio_engine.media_path);
    if !media_path.exists() {
        return Err(AppError::BadRequest("Media directory not found".into()));
    }

    let supported = [".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"];
    let mut new_songs = 0;

    fn walk_dir(dir: &std::path::Path, exts: &[&str]) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(walk_dir(&path, exts));
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = format!(".{}", ext.to_lowercase());
                    if exts.contains(&ext_lower.as_str()) {
                        files.push(path);
                    }
                }
            }
        }
        files
    }

    let audio_files = walk_dir(media_path, &supported);

    for file_path in &audio_files {
        let relative = file_path.strip_prefix(media_path).unwrap_or(file_path);
        let rel_str = relative.to_string_lossy().to_string();

        let existing = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM songs WHERE file_path = ?"
        )
        .bind(&rel_str)
        .fetch_optional(&state.db)
        .await?;

        if existing.is_none() {
            let stem = file_path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut title = stem.clone();
            let mut artist = String::new();

            if let Some(pos) = stem.find(" - ") {
                artist = stem[..pos].to_string();
                title = stem[pos + 3..].to_string();
            }

            let duration_ms = get_duration(file_path).unwrap_or(0);

            let lrc_path = file_path.with_extension("lrc");
            let lyrics_path = if lrc_path.exists() {
                lrc_path.strip_prefix(media_path)
                    .unwrap_or(&lrc_path)
                    .to_string_lossy()
                    .to_string()
            } else {
                String::new()
            };

            let cover_path = find_cover(&file_path, media_path);

            sqlx::query(
                "INSERT INTO songs (title, artist, file_path, lyrics_path, cover_path, duration_ms, filesize) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&title)
            .bind(&artist)
            .bind(&rel_str)
            .bind(&lyrics_path)
            .bind(&cover_path)
            .bind(duration_ms)
            .bind(0_i64)
            .execute(&state.db)
            .await?;

            new_songs += 1;
        }
    }

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'rescan_songs', ?)")
        .bind(admin.id)
        .bind(format!("Found {} new songs", new_songs))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok(format!("Rescan complete. {} new songs added.", new_songs))))
}

// ─── 系统设置 ─────────────────────────────────────────────

/// GET /api/admin/settings — 获取系统设置
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<SettingsResponse>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let station = state.station.read().unwrap_or_else(|e| e.into_inner());
    Ok(Json(ApiResponse::ok(SettingsResponse {
        station_name: station.name.clone(),
        subtitle: station.subtitle.clone(),
        primary_color: station.primary_color.clone(),
        secondary_color: station.secondary_color.clone(),
        bg_color: station.bg_color.clone(),
    })))
}

/// POST /api/admin/settings — 保存系统设置（写入 config.toml）
pub async fn save_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SaveSettingsRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    // 如果提供了管理员密码，更新自己的密码
    if let Some(ref pw) = body.admin_password {
        if !pw.is_empty() {
            if pw.len() < 6 {
                return Err(AppError::BadRequest("密码至少需要6个字符".into()));
            }
            let hash = crate::auth::hash_password(pw)?;

            sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
                .bind(&hash)
                .bind(admin.id)
                .execute(&state.db)
                .await?;
        }
    }

    // 更新内存中的配置并写入文件
    let config_path = std::env::var("RADIO_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());

    let mut toml_value: toml::Value = {
        let content = std::fs::read_to_string(&config_path)
            .unwrap_or_default();
        toml::from_str(&content).unwrap_or(toml::Value::Table(toml::value::Table::new()))
    };

    if let toml::Value::Table(ref mut root) = toml_value {
        let station = root.entry("station")
            .or_insert(toml::Value::Table(toml::value::Table::new()));
        if let toml::Value::Table(ref mut st) = station {
            if let Some(ref v) = body.station_name { st.insert("name".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.subtitle { st.insert("subtitle".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.primary_color { st.insert("primary_color".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.secondary_color { st.insert("secondary_color".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.bg_color { st.insert("bg_color".into(), toml::Value::String(v.clone())); }
        }
    }

    std::fs::write(&config_path, toml::to_string_pretty(&toml_value)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TOML serialize error: {}", e)))?)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Write config error: {}", e)))?;

    // 热更新内存中的 station 配置，无需重启
    {
        let mut station = state.station.write().unwrap_or_else(|e| e.into_inner());
        if let Some(ref v) = body.station_name { station.name = v.clone(); }
        if let Some(ref v) = body.subtitle { station.subtitle = v.clone(); }
        if let Some(ref v) = body.primary_color { station.primary_color = v.clone(); }
        if let Some(ref v) = body.secondary_color { station.secondary_color = v.clone(); }
        if let Some(ref v) = body.bg_color { station.bg_color = v.clone(); }
    }

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'update_settings', ?)")
        .bind(admin.id)
        .bind("Updated system settings")
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("设置已保存，立即生效".into())))
}

// ─── 上传歌曲 ─────────────────────────────────────────────

/// POST /api/admin/upload — 上传音乐文件到媒体目录
pub async fn upload_song(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    let media_path = std::path::PathBuf::from(&state.config.audio_engine.media_path);
    std::fs::create_dir_all(&media_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Create media dir error: {}", e)))?;

    let mut uploaded_filename = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("file").to_string();
        if name != "file" {
            continue;
        }

        let filename = field.file_name()
            .unwrap_or("unknown.mp3")
            .to_string();

        // 清理文件名：移除路径分隔符
        let safe_name = filename
            .replace('/', "_")
            .replace('\\', "_")
            .replace("..", "_");

        let data = field.bytes().await
            .map_err(|e| AppError::BadRequest(format!("读取上传数据失败: {}", e)))?;

        if data.is_empty() {
            return Err(AppError::BadRequest("文件为空".into()));
        }

        let max_size = 100 * 1024 * 1024; // 100 MB
        if data.len() > max_size {
            return Err(AppError::BadRequest("文件大小超过 100MB 限制".into()));
        }

        let dest_path = media_path.join(&safe_name);
        std::fs::write(&dest_path, &data)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("写入文件失败: {}", e)))?;

        uploaded_filename = safe_name.clone();

        // 从文件名提取标题/艺术家
        let stem = dest_path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(safe_name.clone());
        let mut title = stem.clone();
        let mut artist = String::new();
        if let Some(pos) = stem.find(" - ") {
            artist = stem[..pos].to_string();
            title = stem[pos + 3..].to_string();
        }

        let rel_str = safe_name.clone();
        let duration_ms = get_duration(&dest_path).unwrap_or(0);
        let cover_path = find_cover(&dest_path, &media_path);
        let lrc_path = dest_path.with_extension("lrc");
        let lyrics_path = if lrc_path.exists() {
            lrc_path.strip_prefix(&media_path)
                .unwrap_or(&lrc_path)
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };

        sqlx::query(
            "INSERT INTO songs (title, artist, file_path, lyrics_path, cover_path, duration_ms, filesize) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&title)
        .bind(&artist)
        .bind(&rel_str)
        .bind(&lyrics_path)
        .bind(&cover_path)
        .bind(duration_ms)
        .bind(data.len() as i64)
        .execute(&state.db)
        .await?;
    }

    if uploaded_filename.is_empty() {
        return Err(AppError::BadRequest("未找到上传文件字段".into()));
    }

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'upload_song', ?)")
        .bind(admin.id)
        .bind(format!("Uploaded {}", uploaded_filename))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok(format!("上传成功: {}", uploaded_filename))))
}

// ─── 删除歌曲 ─────────────────────────────────────────────

/// DELETE /api/admin/songs/{id} — 删除歌曲（从文件系统和数据库）
pub async fn delete_song(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    // 删除文件（占位歌曲或元数据歌曲可能没有实际文件）
    let file_path = std::path::Path::new(&state.config.audio_engine.media_path).join(&song.file_path);
    if !song.file_path.is_empty() && file_path.exists() {
        std::fs::remove_file(&file_path).ok();
    }

    // 删除歌词文件
    if !song.lyrics_path.is_empty() {
        let lrc_path = std::path::Path::new(&state.config.audio_engine.media_path).join(&song.lyrics_path);
        if lrc_path.exists() {
            std::fs::remove_file(&lrc_path).ok();
        }
    }

    // 删除封面文件
    if !song.cover_path.is_empty() {
        let cover_path = std::path::Path::new(&state.config.audio_engine.media_path).join(&song.cover_path);
        if cover_path.exists() && cover_path != file_path {
            std::fs::remove_file(&cover_path).ok();
        }
    }

    // 从数据库中删除
    sqlx::query("DELETE FROM playlist_songs WHERE song_id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM queue_items WHERE song_id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM favorites WHERE song_id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM songs WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'delete_song', ?)")
        .bind(admin.id)
        .bind(format!("Deleted song {}: {}", id, song.title))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok(format!("已删除: {}", song.title))))
}

// ─── 歌曲列表（管理用，包含文件路径）─────────────────────

/// GET /api/admin/songs — 获取所有歌曲（包含完整信息供管理）
pub async fn list_all_songs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<crate::models::Song>>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let songs = sqlx::query_as::<_, crate::models::Song>(
        "SELECT * FROM songs ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::ok(songs)))
}

// ─── 播放控制（通过 HTTP 向 C++ 引擎发送指令）───────────

/// POST /api/admin/playlist/next — 切到下一首
pub async fn skip_next(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let cmd = AudioCommand {
        cmd_type: "skip".into(),
        song_id: None,
        file_path: None,
    };

    websocket::publish_command(&state, &cmd).await?;

    Ok(Json(ApiResponse::ok("已切到下一首".into())))
}

/// POST /api/admin/playlist/prev — 切到上一首
pub async fn skip_prev(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let cmd = AudioCommand {
        cmd_type: "prev".into(),
        song_id: None,
        file_path: None,
    };

    websocket::publish_command(&state, &cmd).await?;

    Ok(Json(ApiResponse::ok("已切到上一首".into())))
}

// ─── 批量下载（调用 music_dl.py）─────────────────────────

/// POST /api/admin/download — 开始批量下载
pub async fn start_download(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DownloadRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let playlist = body.playlist.trim().to_string();
    if playlist.is_empty() {
        return Err(AppError::BadRequest("歌单内容不能为空".into()));
    }

    // 检查是否已有下载任务在运行
    {
        let status = download_state().lock().unwrap_or_else(|e| e.into_inner());
        if status.running {
            return Err(AppError::BadRequest("已有下载任务在运行中".into()));
        }
    }

    let quality = body.quality.unwrap_or_else(|| "exhigh".into());
    let format = body.format.unwrap_or_else(|| "mp3".into());

    // 将歌单写入临时文件
    let tmpdir = std::env::temp_dir();
    let playlist_file = tmpdir.join("radio_download_playlist.txt");
    std::fs::write(&playlist_file, &playlist)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("写入临时文件失败: {}", e)))?;

    // 更新下载状态
    {
        let mut status = download_state().lock().unwrap_or_else(|e| e.into_inner());
        status.running = true;
        status.log = format!("开始下载...\n音质: {}\n格式: {}\n", quality, format);
    }

    let media_path = state.config.audio_engine.media_path.clone();
    let quality_clone = quality.clone();
    let format_clone = format.clone();
    let playlist_path = playlist_file.clone();

    let dl_path = music_dl_path();
    let settings_path = ncm_secrets_path();
    // 异步执行下载
    tokio::spawn(async move {
        let result = std::process::Command::new("python3")
            .arg(&dl_path)
            .arg(&playlist_path)
            .arg("--output")
            .arg(&media_path)
            .arg("--quality")
            .arg(&quality_clone)
            .arg("--format")
            .arg(&format_clone)
            .arg("--non-interactive")
            .arg("--settings")
            .arg(&settings_path)
            .output();

        let mut status = download_state().lock().unwrap_or_else(|e| e.into_inner());
        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                status.log = format!("{}\n{}", stdout, stderr);
                status.running = false;

                // 清理临时文件
                std::fs::remove_file(&playlist_path).ok();
            }
            Err(e) => {
                status.log = format!("下载失败: {}", e);
                status.running = false;
            }
        }
    });

    Ok(Json(ApiResponse::ok("下载任务已启动".into())))
}

/// GET /api/admin/download/status — 获取下载状态
pub async fn download_status(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DownloadStatus>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let status = download_state().lock().unwrap_or_else(|e| e.into_inner());
    Ok(Json(ApiResponse::ok(status.clone())))
}

// ─── 网易云账号设置 ───────────────────────────────────────

/// 网易云设置文件路径（与 C++ 引擎共用）
/// 默认查找工作目录下的 secrets.json
fn ncm_secrets_path() -> std::path::PathBuf {
    std::env::var("NCM_SECRETS_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("secrets.json"))
}

/// music_dl.py 脚本路径
/// 默认查找工作目录下的 music_dl.py
fn music_dl_path() -> std::path::PathBuf {
    std::env::var("MUSIC_DL_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("music_dl.py"))
}

/// GET /api/admin/ncm — 获取网易云账号状态
pub async fn get_ncm_settings(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let path = ncm_secrets_path();
    if !path.exists() {
        return Ok(Json(ApiResponse::ok(serde_json::json!({
            "configured": false,
            "method": "none",
            "phone_hint": ""
        }))));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("无法读取 secrets.json")))?;

    let secrets: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or(serde_json::Value::Null);

    let configured = secrets.get("ncm_phone").or(secrets.get("ncm_cookie"))
        .map(|v| !v.as_str().unwrap_or("").is_empty())
        .unwrap_or(false);

    let method = if secrets.get("ncm_cookie")
        .map(|v| !v.as_str().unwrap_or("").is_empty())
        .unwrap_or(false)
    {
        "cookie"
    } else if configured { "phone" } else { "none" };

    let phone = secrets.get("ncm_phone")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "configured": configured,
        "method": method,
        "phone_hint": if phone.len() > 4 {
            format!("{}...{}", &phone[..3], &phone[phone.len()-2..])
        } else { phone.to_string() }
    }))))
}

/// POST /api/admin/ncm — 保存网易云账号设置
pub async fn save_ncm_settings(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let path = ncm_secrets_path();
    let mut secrets: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(map) = secrets.as_object_mut() {
        if let Some(cookie) = body.get("cookie").and_then(|v| v.as_str()) {
            if !cookie.is_empty() {
                map.insert("ncm_cookie".into(), serde_json::Value::String(cookie.to_string()));
            } else {
                map.remove("ncm_cookie");
            }
        }
        if let Some(phone) = body.get("phone").and_then(|v| v.as_str()) {
            if !phone.is_empty() {
                map.insert("ncm_phone".into(), serde_json::Value::String(phone.to_string()));
            } else {
                map.remove("ncm_phone");
            }
        }
        if let Some(password) = body.get("password").and_then(|v| v.as_str()) {
            if !password.is_empty() {
                map.insert("ncm_password".into(), serde_json::Value::String(password.to_string()));
            } else {
                map.remove("ncm_password");
            }
        }
    }

    let content = serde_json::to_string_pretty(&secrets)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("序列化失败: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("写入失败: {}", e)))?;

    Ok(Json(ApiResponse::ok("保存成功".into())))
}

/// POST /api/admin/ncm/test — 测试网易云登录
pub async fn test_ncm_login(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    // 调用 music_dl.py 的测试功能
    let result = std::process::Command::new("python3")
        .arg(music_dl_path())
        .arg("--verify-login")
        .arg("--settings")
        .arg(ncm_secrets_path())
        .output();

    match result {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let combined = if stderr.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                format!("{}\n{}", stdout.trim(), stderr.trim())
            };
            let success = output.status.success();
            Ok(Json(ApiResponse::ok(serde_json::json!({
                "success": success,
                "output": if combined.is_empty() {
                    if success { "登录成功".to_string() } else { "登录失败".to_string() }
                } else { combined },
            }))))
        }
        Err(e) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": false,
            "output": format!("执行失败: {}", e),
        })))),
    }
}

// ─── 退出登录（管理面板）─────────────────────────────────

/// POST /api/admin/logout — 退出登录（前端清除 JWT token 即可，此为预留端点）
pub async fn logout(
    State(_state): State<Arc<AppState>>,
    _headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::ok("已退出".into())))
}
