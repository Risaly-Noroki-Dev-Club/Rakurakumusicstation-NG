/// 用户播放列表路由：个人播放列表的增删改查。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{
    AddSongToPlaylistRequest, ApiResponse, CreatePlaylistRequest, PlaylistWithCount, SongSummary,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn playlist_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_my_playlists).post(create_playlist))
        .route("/{id}", get(get_playlist).delete(delete_playlist))
        .route("/{id}/songs", post(add_song).delete(remove_song))
}

/// 从请求头中提取已认证用户的辅助函数（使用共享 auth 模块）。
async fn get_user(state: &AppState, headers: &HeaderMap) -> Result<crate::auth::AuthUser, AppError> {
    auth::require_auth_from_headers(headers, &state.db, &state.jwt_secret).await
}

/// GET /api/playlists — 列出当前用户的播放列表
async fn list_my_playlists(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<PlaylistWithCount>>>, AppError> {
    let user = get_user(&state, &headers).await?;

    let playlists = sqlx::query_as::<_, (i64, i64, String, bool, String)>(
        r#"
        SELECT p.id, p.user_id, p.name, p.is_public, p.created_at
        FROM playlists p
        WHERE p.user_id = ?
        ORDER BY p.created_at DESC
        "#
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let mut result = Vec::new();
    for (id, user_id, name, is_public, created_at_str) in playlists {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM playlist_songs WHERE playlist_id = ?"
        )
        .bind(id)
        .fetch_one(&state.db)
        .await?;

        result.push(PlaylistWithCount {
            id,
            user_id,
            name,
            is_public,
            song_count: count.0,
            created_at: chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_default(),
        });
    }

    Ok(Json(ApiResponse::ok(result)))
}

/// POST /api/playlists — 创建新播放列表
async fn create_playlist(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreatePlaylistRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user = get_user(&state, &headers).await?;

    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("Playlist name cannot be empty".into()));
    }

    let result = sqlx::query(
        "INSERT INTO playlists (user_id, name, is_public) VALUES (?, ?, ?)"
    )
    .bind(user.id)
    .bind(req.name.trim())
    .bind(req.is_public)
    .execute(&state.db)
    .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "id": result.last_insert_rowid(),
        "name": req.name.trim(),
    }))))
}

/// GET /api/playlists/{id} — 获取播放列表详情及歌曲
async fn get_playlist(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user = get_user(&state, &headers).await?;

    // 获取播放列表
    let playlist = sqlx::query_as::<_, crate::models::Playlist>(
        "SELECT * FROM playlists WHERE id = ?"
    )
    .bind(playlist_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Playlist not found".into()))?;

    // 检查所有权（公开播放列表除外）
    if playlist.user_id != user.id && !playlist.is_public {
        return Err(AppError::Forbidden("Not your playlist".into()));
    }

    // 获取歌曲
    let songs = sqlx::query_as::<_, crate::models::Song>(
        r#"
        SELECT s.* FROM songs s
        JOIN playlist_songs ps ON ps.song_id = s.id
        WHERE ps.playlist_id = ?
        ORDER BY ps.position ASC
        "#
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let song_summaries: Vec<SongSummary> = songs.into_iter().map(SongSummary::from).collect();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "id": playlist.id,
        "name": playlist.name,
        "is_public": playlist.is_public,
        "owner_id": playlist.user_id,
        "songs": song_summaries,
        "song_count": song_summaries.len(),
    }))))
}

/// DELETE /api/playlists/{id}
async fn delete_playlist(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = get_user(&state, &headers).await?;

    let playlist = sqlx::query_as::<_, crate::models::Playlist>(
        "SELECT * FROM playlists WHERE id = ?"
    )
    .bind(playlist_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Playlist not found".into()))?;

    if playlist.user_id != user.id {
        return Err(AppError::Forbidden("Not your playlist".into()));
    }

    sqlx::query("DELETE FROM playlists WHERE id = ?")
        .bind(playlist_id)
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Playlist deleted".into())))
}

/// POST /api/playlists/{id}/songs — 添加歌曲到播放列表
async fn add_song(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<i64>,
    headers: HeaderMap,
    Json(req): Json<AddSongToPlaylistRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = get_user(&state, &headers).await?;

    let playlist = sqlx::query_as::<_, crate::models::Playlist>(
        "SELECT * FROM playlists WHERE id = ?"
    )
    .bind(playlist_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Playlist not found".into()))?;

    if playlist.user_id != user.id {
        return Err(AppError::Forbidden("Not your playlist".into()));
    }

    // 检查歌曲是否存在
    sqlx::query_as::<_, (i64,)>("SELECT id FROM songs WHERE id = ?")
        .bind(req.song_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    // 获取下一个位置
    let max_pos: Option<(i32,)> = sqlx::query_as(
        "SELECT MAX(position) FROM playlist_songs WHERE playlist_id = ?"
    )
    .bind(playlist_id)
    .fetch_optional(&state.db)
    .await?;

    let next_pos = max_pos.map(|(p,)| p + 1).unwrap_or(0);

    // 忽略重复项（INSERT OR IGNORE 行为）
    let result = sqlx::query(
        "INSERT OR IGNORE INTO playlist_songs (playlist_id, song_id, position) VALUES (?, ?, ?)"
    )
    .bind(playlist_id)
    .bind(req.song_id)
    .bind(next_pos)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::Conflict("Song already in playlist".into()));
    }

    Ok(Json(ApiResponse::ok("Song added to playlist".into())))
}

/// DELETE /api/playlists/{id}/songs — 从播放列表中移除歌曲
async fn remove_song(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<i64>,
    headers: HeaderMap,
    Json(req): Json<AddSongToPlaylistRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = get_user(&state, &headers).await?;

    let playlist = sqlx::query_as::<_, crate::models::Playlist>(
        "SELECT * FROM playlists WHERE id = ?"
    )
    .bind(playlist_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Playlist not found".into()))?;

    if playlist.user_id != user.id {
        return Err(AppError::Forbidden("Not your playlist".into()));
    }

    sqlx::query("DELETE FROM playlist_songs WHERE playlist_id = ? AND song_id = ?")
        .bind(playlist_id)
        .bind(req.song_id)
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Song removed from playlist".into())))
}
