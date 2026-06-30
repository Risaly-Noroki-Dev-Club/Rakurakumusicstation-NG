/// 歌曲管理路由。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::routes::admin::get_admin;
use crate::services::metadata::{find_cover, get_duration, parse_artist_title};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

/// GET /api/admin/songs — 获取所有歌曲
pub async fn list_all_songs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<crate::models::Song>>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let songs =
        sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?;

    Ok(Json(ApiResponse::ok(songs)))
}

/// DELETE /api/admin/songs/{id} — 删除歌曲
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

    let file_path =
        std::path::Path::new(&state.config.audio_engine.media_path).join(&song.file_path);
    if !song.file_path.is_empty() && file_path.exists() {
        std::fs::remove_file(&file_path).ok();
    }

    if !song.lyrics_path.is_empty() {
        let lrc_path =
            std::path::Path::new(&state.config.audio_engine.media_path).join(&song.lyrics_path);
        if lrc_path.exists() {
            std::fs::remove_file(&lrc_path).ok();
        }
    }

    if !song.cover_path.is_empty() {
        let cover_path =
            std::path::Path::new(&state.config.audio_engine.media_path).join(&song.cover_path);
        if cover_path.exists() && cover_path != file_path {
            std::fs::remove_file(&cover_path).ok();
        }
    }

    {
        let _queue_guard = state.queue_sync.lock().await;

        sqlx::query("DELETE FROM playlist_songs WHERE song_id = ?")
            .bind(id)
            .execute(&state.db)
            .await?;
        sqlx::query("DELETE FROM queue_items WHERE song_id = ?")
            .bind(id)
            .execute(&state.db)
            .await?;
        state.player_handle.remove_request_by_song_id(id);
        sqlx::query("DELETE FROM favorites WHERE song_id = ?")
            .bind(id)
            .execute(&state.db)
            .await?;
        sqlx::query("DELETE FROM songs WHERE id = ?")
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'delete_song', ?)")
        .bind(admin.id)
        .bind(format!("Deleted song {}: {}", id, song.title))
        .execute(&state.db)
        .await?;

    state
        .player_handle
        .send_command(radio_engine::types::AudioCommand {
            cmd_type: radio_engine::types::AudioCommandType::ReloadQueue,
            song_id: None,
            file_path: None,
        });

    Ok(Json(ApiResponse::ok(format!("已删除: {}", song.title))))
}

/// POST /api/admin/rescan-songs — 重新扫描媒体目录
pub async fn rescan_songs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    let media_path = std::path::Path::new(&state.config.audio_engine.media_path);
    if !media_path.exists() {
        return Err(AppError::BadRequest("Media directory not found".into()));
    }

    let supported = radio_engine::config::SUPPORTED_FORMATS;
    let mut new_songs = 0;

    fn walk_dir(dir: &std::path::Path, exts: &[&str]) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(walk_dir(&path, exts));
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if exts.iter().any(|f| *f == ext_lower) {
                        files.push(path);
                    }
                }
            }
        }
        files
    }

    let audio_files = walk_dir(media_path, supported);

    for file_path in &audio_files {
        let relative = file_path.strip_prefix(media_path).unwrap_or(file_path);
        let rel_str = relative.to_string_lossy().to_string();

        let existing = sqlx::query_as::<_, (i64,)>("SELECT id FROM songs WHERE file_path = ?")
            .bind(&rel_str)
            .fetch_optional(&state.db)
            .await?;

        if existing.is_none() {
            let stem = file_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let (artist, title) = parse_artist_title(&stem);
            let duration_ms = get_duration(file_path).unwrap_or(0);

            let lrc_path = file_path.with_extension("lrc");
            let lyrics_path = if lrc_path.exists() {
                lrc_path
                    .strip_prefix(media_path)
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

    if new_songs > 0 {
        state
            .player_handle
            .send_command(radio_engine::types::AudioCommand {
                cmd_type: radio_engine::types::AudioCommandType::ReloadQueue,
                song_id: None,
                file_path: None,
            });
    }

    Ok(Json(ApiResponse::ok(format!(
        "Rescan complete. {} new songs added.",
        new_songs
    ))))
}
