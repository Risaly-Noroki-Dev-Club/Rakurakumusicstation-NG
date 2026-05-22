/// 上传路由（管理员）。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::routes::admin::get_admin;
use crate::services::metadata::{find_cover, get_duration, parse_artist_title, sanitize_filename};
use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

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

        let filename = field.file_name().unwrap_or("unknown.mp3").to_string();

        let safe_name = sanitize_filename(&filename);

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("读取上传数据失败: {}", e)))?;

        if data.is_empty() {
            return Err(AppError::BadRequest("文件为空".into()));
        }

        let max_size = 100 * 1024 * 1024;
        if data.len() > max_size {
            return Err(AppError::BadRequest("文件大小超过 100MB 限制".into()));
        }

        let dest_path = media_path.join(&safe_name);
        std::fs::write(&dest_path, &data)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("写入文件失败: {}", e)))?;

        uploaded_filename = safe_name.clone();

        let stem = dest_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(safe_name.clone());
        let (artist, title) = parse_artist_title(&stem);

        let rel_str = safe_name.clone();
        let duration_ms = get_duration(&dest_path).unwrap_or(0);
        let cover_path = find_cover(&dest_path, &media_path);
        let lrc_path = dest_path.with_extension("lrc");
        let lyrics_path = if lrc_path.exists() {
            lrc_path
                .strip_prefix(&media_path)
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

    // 让引擎重扫媒体目录，否则空文件夹起服务时上传后引擎 play_queue 仍然是空的。
    state
        .player_handle
        .send_command(radio_engine::types::AudioCommand {
            cmd_type: radio_engine::types::AudioCommandType::ReloadQueue,
            song_id: None,
            file_path: None,
        });

    Ok(Json(ApiResponse::ok(format!(
        "上传成功: {}",
        uploaded_filename
    ))))
}
