/// 歌曲库路由：搜索和获取歌曲详情。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, PaginatedResponse, SearchQuery, SongSummary};
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::Response,
    routing::get,
    Json, Router,
};
use std::sync::Arc;

pub fn song_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/cover", get(get_song_cover))
        .route("/:id", get(get_song))
        .route("/", get(search_songs))
}

/// GET /api/songs?q=search&limit=20&offset=0
pub async fn search_songs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<SongSummary>>>, AppError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let search = query.q.unwrap_or_default().trim().to_string();

    let (songs, total): (Vec<crate::models::Song>, i64) = if search.is_empty() {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM songs")
            .fetch_one(&state.db)
            .await?;

        let songs = sqlx::query_as::<_, crate::models::Song>(
            "SELECT * FROM songs ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

        (songs, total.0)
    } else {
        let pattern = format!("%{}%", search);
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ?"
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&state.db)
        .await?;

        let songs = sqlx::query_as::<_, crate::models::Song>(
            "SELECT * FROM songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

        (songs, total.0)
    };

    let data: Vec<SongSummary> = songs.into_iter().map(SongSummary::from).collect();

    Ok(Json(ApiResponse::ok(PaginatedResponse {
        total,
        limit,
        offset,
        data,
    })))
}

/// GET /api/songs/{id}
pub async fn get_song(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<crate::models::Song>>, AppError> {
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    Ok(Json(ApiResponse::ok(song)))
}

/// 缺省封面占位 SVG — 简约音乐符图标
const DEFAULT_COVER_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120" fill="none">
  <rect width="120" height="120" rx="8" fill="#e8e8f0"/>
  <path d="M42 85V42l36-8v43" stroke="#999" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>
  <circle cx="38" cy="85" r="7" fill="#bbb"/>
  <circle cx="74" cy="77" r="7" fill="#bbb"/>
</svg>"##;

/// GET /api/songs/{id}/cover — 返回封面图片（JPEG/PNG/SVG 二进制数据）
pub async fn get_song_cover(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    if song.cover_path.is_empty() {
        return Ok(Response::builder()
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(axum::body::Body::from(DEFAULT_COVER_SVG))
            .unwrap());
    }

    let cover_full = std::path::Path::new(&state.config.audio_engine.media_path)
        .join(&song.cover_path);

    let data = std::fs::read(&cover_full)
        .map_err(|_| {
            // 文件丢失时也回退到缺省封面
            DEFAULT_COVER_SVG.to_string()
        });

    match data {
        Ok(bytes) => {
            let mime = match cover_full.extension().and_then(|e| e.to_str()) {
                Some("png") => "image/png",
                _ => "image/jpeg",
            };
            Ok(Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(axum::body::Body::from(bytes))
                .unwrap())
        }
        Err(_) => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(axum::body::Body::from(DEFAULT_COVER_SVG))
            .unwrap()),
    }
}
