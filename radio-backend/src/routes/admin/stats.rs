/// 统计与日志路由。
use crate::db::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::routes::admin::get_admin;
use axum::{extract::State, http::HeaderMap, Json};
use std::sync::Arc;

/// GET /api/admin/stats — 系统统计
pub async fn stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM device_users")
        .fetch_one(&state.db)
        .await?;

    let song_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM songs")
        .fetch_one(&state.db)
        .await?;

    let queue_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM queue_items WHERE status IN ('pending', 'playing')")
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
pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let logs = sqlx::query_as::<_, crate::models::AdminLog>(
        "SELECT * FROM admin_log ORDER BY created_at DESC LIMIT 100",
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
