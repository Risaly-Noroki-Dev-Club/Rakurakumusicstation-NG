/// 队列路由：查看、添加、移动、移除、跳过、历史记录、正在播放。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{AddToQueueRequest, ApiResponse, MoveQueueItemRequest, NowPlaying, PlaybackState};
use crate::queue_manager;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn queue_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_queue).post(add_to_queue))
        .route("/{id}", delete(remove_queue_item))
        .route("/{id}/move", post(move_queue_item))
        .route("/skip", post(skip_current))
        .route("/history", get(get_history))
}

/// 从请求头中提取已认证用户的辅助函数。
async fn get_user(state: &AppState, headers: &HeaderMap) -> Result<crate::auth::AuthUser, AppError> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let claims = auth::validate_token(token, &state.jwt_secret)?;

    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(claims.sub.parse::<i64>().unwrap_or(0))
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.is_banned() {
        return Err(AppError::Banned);
    }

    Ok(crate::auth::AuthUser {
        id: user.id,
        username: user.username,
        role: user.role,
    })
}

/// 辅助函数：可选认证（已登录返回 Some，访客返回 None）。
async fn get_optional_user(state: &AppState, headers: &HeaderMap) -> Option<crate::auth::AuthUser> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))?;

    let claims = auth::validate_token(token, &state.jwt_secret).ok()?;

    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(claims.sub.parse::<i64>().unwrap_or(0))
        .fetch_optional(&state.db)
        .await
        .ok()??;

    if user.is_banned() {
        return None;
    }

    Some(crate::auth::AuthUser {
        id: user.id,
        username: user.username,
        role: user.role,
    })
}

/// GET /api/queue — 获取当前队列（公开，无需认证）
async fn get_queue(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<crate::models::QueueItemDisplay>>>, AppError> {
    let items = queue_manager::get_queue_display(&state.db).await?;
    Ok(Json(ApiResponse::ok(items)))
}

/// POST /api/queue — 添加歌曲到队列（需要认证）
async fn add_to_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AddToQueueRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user = get_user(&state, &headers).await?;

    let item_id = queue_manager::add_to_queue(&state, req.song_id, user.id, &user.username).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "queue_item_id": item_id,
        "message": "Song added to queue",
    }))))
}

/// DELETE /api/queue/{id} — 从队列中移除项目（仅限管理员）
async fn remove_queue_item(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = get_user(&state, &headers).await?;
    auth::require_admin(&user)?;

    queue_manager::remove_queue_item(&state.db, item_id).await?;

    // 记录管理操作
    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'remove_queue', ?)")
        .bind(user.id)
        .bind(format!("Removed queue item {}", item_id))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Queue item removed".into())))
}

/// POST /api/queue/{id}/move — 移动队列项目到新位置（仅限管理员）
async fn move_queue_item(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<i64>,
    headers: HeaderMap,
    Json(req): Json<MoveQueueItemRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = get_user(&state, &headers).await?;
    auth::require_admin(&user)?;

    queue_manager::move_queue_item(&state.db, item_id, req.new_position).await?;

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'move_queue', ?)")
        .bind(user.id)
        .bind(format!("Moved item {} to position {}", item_id, req.new_position))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Queue item moved".into())))
}

/// POST /api/queue/skip — 跳过当前歌曲（仅限管理员）
async fn skip_current(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = get_user(&state, &headers).await?;
    auth::require_admin(&user)?;

    queue_manager::skip_current(&state).await?;

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'skip_track', 'Skipped current track')")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Track skipped".into())))
}

/// GET /api/queue/history — 最近播放历史（公开）
async fn get_history(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let history = queue_manager::get_history(&state.db, 20).await?;
    Ok(Json(ApiResponse::ok(history)))
}

/// GET /api/now-playing — 当前曲目信息（公开）
/// 此端点将 Redis 中最近的播放状态与数据库歌曲信息结合起来。
pub async fn now_playing(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<NowPlaying>>, AppError> {
    // 尝试从 queue_items 获取当前正在播放的歌曲
    let playing = sqlx::query_as::<_, crate::models::QueueItem>(
        "SELECT * FROM queue_items WHERE status = 'playing' ORDER BY position ASC LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await?;

    let song = match &playing {
        Some(item) => {
            sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
                .bind(item.song_id)
                .fetch_optional(&state.db)
                .await?
        }
        None => None,
    };

    Ok(Json(ApiResponse::ok(NowPlaying {
        song: song.map(|s| s.into()),
        position_ms: 0,
        duration_ms: song.as_ref().map(|s| s.duration_ms).unwrap_or(0),
        lyrics_line: None,
        lyrics_text: None,
        started_at: playing.as_ref().map(|p| p.played_at.map(|t| t.to_string()).unwrap_or_default()),
        stream_url: format!("{}:{}/stream", state.config.audio_engine.base_url, 2240),
        file_url: song.as_ref().map(|s| {
            format!("{}:{}/file/{}", state.config.audio_engine.base_url, 2240, s.id)
        }),
    })))
}
