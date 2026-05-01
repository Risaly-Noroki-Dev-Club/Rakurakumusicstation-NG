/// 管理员专用路由：用户管理、系统统计、歌曲管理。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/ban", post(ban_user))
        .route("/users/{id}/unban", post(unban_user))
        .route("/stats", get(stats))
        .route("/logs", get(get_logs))
        .route("/rescan-songs", post(rescan_songs))
}

/// 从请求头中提取已认证管理员用户的辅助函数（使用共享 auth 模块）。
async fn get_admin(state: &AppState, headers: &HeaderMap) -> Result<crate::auth::AuthUser, AppError> {
    auth::require_admin_from_headers(headers, &state.db, &state.jwt_secret).await
}

/// GET /api/admin/users — 列出所有用户
async fn list_users(
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
async fn ban_user(
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
async fn unban_user(
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

/// GET /api/admin/stats — 系统统计
async fn stats(
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

/// GET /api/admin/logs — 获取最近的管理日志
async fn get_logs(
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

/// POST /api/admin/rescan-songs — 扫描媒体目录以查找新歌曲
/// 向 C++ 音频引擎发布重新扫描的命令。
/// 在音频文件所在目录查找封面图片
fn find_cover(audio_path: &std::path::Path, media_root: &std::path::Path) -> String {
    let cover_names = ["cover.jpg", "cover.png", "cover.jpeg",
                       "folder.jpg", "folder.png",
                       "album.jpg", "album.png",
                       "front.jpg", "front.png",
                       "AlbumCover.jpg", "AlbumCover.png"];
    let parent = audio_path.parent().unwrap_or(audio_path);

    for name in &cover_names {
        let candidate = parent.join(name);
        if candidate.exists() {
            return candidate.strip_prefix(media_root)
                .unwrap_or(&candidate)
                .to_string_lossy()
                .to_string();
        }
    }

    // 也检查音频文件同名的封面: song.mp3 -> song.jpg
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

async fn rescan_songs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    // 扫描媒体目录
    let media_path = std::path::Path::new(&state.config.audio_engine.media_path);
    if !media_path.exists() {
        return Err(AppError::BadRequest("Media directory not found".into()));
    }

    // 查找音频文件
    let supported = [".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"];
    let mut new_songs = 0;

    // 递归遍历媒体目录
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
        let _filename = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // 检查是否已在数据库中
        let existing = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM songs WHERE file_path = ?"
        )
        .bind(&rel_str)
        .fetch_optional(&state.db)
        .await?;

        if existing.is_none() {
            // 从文件名提取标题/艺术家："艺术家 - 标题.扩展名"
            let stem = file_path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut title = stem.clone();
            let mut artist = String::new();

            if let Some(pos) = stem.find(" - ") {
                artist = stem[..pos].to_string();
                title = stem[pos + 3..].to_string();
            }

            // 尝试通过 ffprobe 获取时长
            let duration_ms = get_duration(file_path).unwrap_or(0);

            // 查找同名的 .lrc 文件
            let lrc_path = file_path.with_extension("lrc");
            let lyrics_path = if lrc_path.exists() {
                lrc_path.strip_prefix(media_path)
                    .unwrap_or(&lrc_path)
                    .to_string_lossy()
                    .to_string()
            } else {
                String::new()
            };

            // 查找封面图片 (cover.jpg, folder.jpg, etc.)
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

/// 通过 ffprobe 获取音频时长（fork+exec）。
fn get_duration(path: &std::path::Path) -> Option<i64> {
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
