/// 收藏路由：收藏/取消收藏歌曲。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, SongSummary};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn favorites_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_favorites))
        .route("/{song_id}", post(add_favorite).delete(remove_favorite))
}

/// GET /api/favorites — 列出当前用户的收藏歌曲
async fn list_favorites(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<SongSummary>>>, AppError> {
    let user = auth::require_auth_from_headers(&headers, &state.db, &state.jwt_secret).await?;

    let songs = sqlx::query_as::<_, crate::models::Song>(
        r#"
        SELECT s.* FROM songs s
        JOIN favorites f ON f.song_id = s.id
        WHERE f.user_id = ?
        ORDER BY f.created_at DESC
        "#
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let summaries: Vec<SongSummary> = songs.into_iter().map(SongSummary::from).collect();
    Ok(Json(ApiResponse::ok(summaries)))
}

/// POST /api/favorites/{song_id} — 收藏歌曲
async fn add_favorite(
    State(state): State<Arc<AppState>>,
    Path(song_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = auth::require_auth_from_headers(&headers, &state.db, &state.jwt_secret).await?;

    // 验证歌曲存在
    sqlx::query_as::<_, (i64,)>("SELECT id FROM songs WHERE id = ?")
        .bind(song_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    // 忽略重复 (INSERT OR IGNORE)
    let result = sqlx::query(
        "INSERT OR IGNORE INTO favorites (user_id, song_id) VALUES (?, ?)"
    )
    .bind(user.id)
    .bind(song_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(Json(ApiResponse::ok("Already favorited".into())));
    }

    Ok(Json(ApiResponse::ok("Favorited".into())))
}

/// DELETE /api/favorites/{song_id} — 取消收藏
async fn remove_favorite(
    State(state): State<Arc<AppState>>,
    Path(song_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = auth::require_auth_from_headers(&headers, &state.db, &state.jwt_secret).await?;

    sqlx::query("DELETE FROM favorites WHERE user_id = ? AND song_id = ?")
        .bind(user.id)
        .bind(song_id)
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Unfavorited".into())))
}
