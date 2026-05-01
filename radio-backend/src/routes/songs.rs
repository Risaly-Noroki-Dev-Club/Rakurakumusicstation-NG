/// 歌曲库路由：搜索和获取歌曲详情。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, PaginatedResponse, SearchQuery, SongSummary};
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

pub fn song_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(search_songs))
        .route("/{id}", get(get_song))
}

/// GET /api/songs?q=search&limit=20&offset=0
async fn search_songs(
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
async fn get_song(
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
