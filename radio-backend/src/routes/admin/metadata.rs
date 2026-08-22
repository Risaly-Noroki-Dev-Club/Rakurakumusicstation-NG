use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::services::metadata_jobs::{self, CreateMetadataJob, MetadataCandidate};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use super::get_admin;

#[derive(Debug, Deserialize)]
pub struct JobItemsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MetadataPatch {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub lock_title: Option<bool>,
    pub lock_artist: Option<bool>,
    pub lock_album: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CandidateApply {
    pub candidate: MetadataCandidate,
}

#[derive(Debug, Deserialize)]
pub struct LyricsPatch {
    pub content: String,
}

pub async fn create_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateMetadataJob>,
) -> Result<Json<ApiResponse<metadata_jobs::MetadataJob>>, AppError> {
    get_admin(&state, &headers).await?;
    let job = state.metadata_jobs.create(&state.db, body).await?;
    Ok(Json(ApiResponse::ok(job)))
}

pub async fn get_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<metadata_jobs::MetadataJob>>, AppError> {
    get_admin(&state, &headers).await?;
    Ok(Json(ApiResponse::ok(
        metadata_jobs::get_job(&state.db, &id).await?,
    )))
}

pub async fn get_items(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<JobItemsQuery>,
) -> Result<Json<ApiResponse<Vec<metadata_jobs::MetadataJobItem>>>, AppError> {
    get_admin(&state, &headers).await?;
    let items = metadata_jobs::get_job_items(
        &state.db,
        &id,
        query.status.as_deref(),
        query.limit.unwrap_or(100).clamp(1, 500),
        query.offset.unwrap_or(0).max(0),
    )
    .await?;
    Ok(Json(ApiResponse::ok(items)))
}

pub async fn cancel_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    get_admin(&state, &headers).await?;
    metadata_jobs::cancel_job(&state.db, &id).await?;
    Ok(Json(ApiResponse::ok("任务已取消".into())))
}

pub async fn retry_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<metadata_jobs::MetadataJob>>, AppError> {
    get_admin(&state, &headers).await?;
    Ok(Json(ApiResponse::ok(
        metadata_jobs::retry_job(&state.db, &state.metadata_jobs, &id).await?,
    )))
}

pub async fn candidates(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<MetadataCandidate>>>, AppError> {
    get_admin(&state, &headers).await?;
    Ok(Json(ApiResponse::ok(
        metadata_jobs::candidates_for_song(&state.db, song_id).await?,
    )))
}

pub async fn apply_candidate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
    Json(body): Json<CandidateApply>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    get_admin(&state, &headers).await?;
    let media_root = std::path::Path::new(&state.config.audio_engine.media_path);
    metadata_jobs::apply_candidate(&state.db, media_root, song_id, &body.candidate).await?;
    state
        .metadata_revision_signal
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Json(ApiResponse::ok("候选元数据已应用".into())))
}

pub async fn fields(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
) -> Result<Json<ApiResponse<Vec<metadata_jobs::MetadataFieldState>>>, AppError> {
    get_admin(&state, &headers).await?;
    Ok(Json(ApiResponse::ok(
        metadata_jobs::metadata_fields(&state.db, song_id).await?,
    )))
}

pub async fn patch_song(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
    Json(body): Json<MetadataPatch>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    get_admin(&state, &headers).await?;
    let existing = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id=?")
        .bind(song_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;
    let title = body.title.unwrap_or(existing.title);
    let artist = body.artist.unwrap_or(existing.artist);
    let album = body.album.unwrap_or(existing.album);
    sqlx::query("UPDATE songs SET title=?,artist=?,album=?,metadata_source='manual',metadata_revision=metadata_revision+1 WHERE id=?")
        .bind(title).bind(artist).bind(album).bind(song_id).execute(&state.db).await?;
    for (field, lock) in [
        ("title", body.lock_title),
        ("artist", body.lock_artist),
        ("album", body.lock_album),
    ] {
        if lock.is_some() {
            sqlx::query("INSERT INTO song_metadata_fields (song_id,field_name,source,locked) VALUES (?,?,'manual',?) ON CONFLICT(song_id,field_name) DO UPDATE SET source='manual',locked=excluded.locked,updated_at=datetime('now')")
                .bind(song_id).bind(field).bind(lock.unwrap_or(false)).execute(&state.db).await?;
        } else {
            sqlx::query("INSERT INTO song_metadata_fields (song_id,field_name,source,locked) VALUES (?,?,'manual',1) ON CONFLICT(song_id,field_name) DO UPDATE SET source='manual',locked=1,updated_at=datetime('now')")
                .bind(song_id).bind(field).execute(&state.db).await?;
        }
    }
    state
        .metadata_revision_signal
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Json(ApiResponse::ok("元数据已保存，字段已锁定".into())))
}

pub async fn put_lyrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
    Json(body): Json<LyricsPatch>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    get_admin(&state, &headers).await?;
    if body.content.trim().is_empty() {
        return Err(AppError::BadRequest("歌词不能为空".into()));
    }
    sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id=?")
        .bind(song_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;
    let path = crate::services::metadata::cache_lyrics(
        std::path::Path::new(&state.config.audio_engine.media_path),
        song_id,
        &body.content,
    )
    .await?;
    sqlx::query("UPDATE songs SET lyrics_path=?,metadata_source='manual',metadata_revision=metadata_revision+1 WHERE id=?").bind(path).bind(song_id).execute(&state.db).await?;
    sqlx::query("INSERT INTO song_metadata_fields (song_id,field_name,source,locked) VALUES (?, 'lyrics','manual',1) ON CONFLICT(song_id,field_name) DO UPDATE SET source='manual',locked=1,updated_at=datetime('now')").bind(song_id).execute(&state.db).await?;
    state
        .metadata_revision_signal
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Json(ApiResponse::ok("歌词已保存".into())))
}

pub async fn delete_lyrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    get_admin(&state, &headers).await?;
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id=?")
        .bind(song_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;
    if !song.lyrics_path.is_empty() {
        let _ = tokio::fs::remove_file(
            std::path::Path::new(&state.config.audio_engine.media_path).join(&song.lyrics_path),
        )
        .await;
    }
    sqlx::query("UPDATE songs SET lyrics_path='',metadata_source='manual',metadata_revision=metadata_revision+1 WHERE id=?").bind(song_id).execute(&state.db).await?;
    sqlx::query("INSERT INTO song_metadata_fields (song_id,field_name,source,locked) VALUES (?, 'lyrics','manual',1) ON CONFLICT(song_id,field_name) DO UPDATE SET source='manual',locked=1,updated_at=datetime('now')").bind(song_id).execute(&state.db).await?;
    state
        .metadata_revision_signal
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Json(ApiResponse::ok("歌词已移除".into())))
}

pub async fn put_cover(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    get_admin(&state, &headers).await?;
    let mut bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name().unwrap_or("file") == "file" {
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?,
            );
            break;
        }
    }
    let bytes = bytes.ok_or_else(|| AppError::BadRequest("缺少封面文件".into()))?;
    if bytes.is_empty() || bytes.len() > 10 * 1024 * 1024 {
        return Err(AppError::BadRequest("封面大小无效".into()));
    }
    let valid_image = bytes.starts_with(&[0xff, 0xd8, 0xff])
        || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"));
    if !valid_image {
        return Err(AppError::BadRequest("封面格式不受支持".into()));
    }
    let media = std::path::Path::new(&state.config.audio_engine.media_path);
    let dir = media.join(".covers");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let relative = format!(".covers/{}.jpg", song_id);
    let temporary = dir.join(format!("{}.jpg.part", song_id));
    tokio::fs::write(&temporary, &bytes)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    tokio::fs::rename(&temporary, media.join(&relative))
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    sqlx::query("UPDATE songs SET cover_path=?,metadata_source='manual',metadata_revision=metadata_revision+1 WHERE id=?").bind(&relative).bind(song_id).execute(&state.db).await?;
    sqlx::query("INSERT INTO song_metadata_fields (song_id,field_name,source,locked) VALUES (?, 'cover','manual',1) ON CONFLICT(song_id,field_name) DO UPDATE SET source='manual',locked=1,updated_at=datetime('now')").bind(song_id).execute(&state.db).await?;
    state
        .metadata_revision_signal
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Json(ApiResponse::ok("封面已保存".into())))
}

pub async fn delete_cover(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(song_id): Path<i64>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    get_admin(&state, &headers).await?;
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id=?")
        .bind(song_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;
    if !song.cover_path.is_empty() {
        let _ = tokio::fs::remove_file(
            std::path::Path::new(&state.config.audio_engine.media_path).join(&song.cover_path),
        )
        .await;
    }
    sqlx::query("UPDATE songs SET cover_path='',metadata_source='manual',metadata_revision=metadata_revision+1 WHERE id=?").bind(song_id).execute(&state.db).await?;
    sqlx::query("INSERT INTO song_metadata_fields (song_id,field_name,source,locked) VALUES (?, 'cover','manual',1) ON CONFLICT(song_id,field_name) DO UPDATE SET source='manual',locked=1,updated_at=datetime('now')").bind(song_id).execute(&state.db).await?;
    state
        .metadata_revision_signal
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Json(ApiResponse::ok("封面已移除".into())))
}
