/// 设备用户管理路由。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, Role, SetRoleRequest};
use crate::routes::admin::get_admin;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

/// GET /api/admin/users — 列出所有设备用户
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<crate::models::DeviceUser>>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let users = sqlx::query_as::<_, crate::models::DeviceUser>(
        "SELECT * FROM device_users ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::ok(users)))
}

/// POST /api/admin/users/{id}/ban — 封禁设备用户
pub async fn ban_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    if user_id == admin.id {
        return Err(AppError::BadRequest("Cannot ban yourself".into()));
    }

    sqlx::query("UPDATE device_users SET banned_until = datetime('now', '+100 years') WHERE id = ?")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'ban_user', ?)")
        .bind(admin.id)
        .bind(format!("Banned device user {}", user_id))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Device user banned".into())))
}

/// POST /api/admin/users/{id}/unban
pub async fn unban_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    sqlx::query("UPDATE device_users SET banned_until = NULL WHERE id = ?")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'unban_user', ?)")
        .bind(admin.id)
        .bind(format!("Unbanned device user {}", user_id))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("Device user unbanned".into())))
}

/// PUT /api/admin/users/{id}/role — 更改设备用户角色
pub async fn set_user_role(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
    Json(body): Json<SetRoleRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    if user_id == admin.id {
        return Err(AppError::BadRequest("Cannot change your own role".into()));
    }

    let target = sqlx::query_as::<_, crate::models::DeviceUser>(
        "SELECT * FROM device_users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Device user not found".into()))?;

    let old_role = match target.role.as_str() {
        "admin" => Role::Admin,
        _ => Role::User,
    };
    if old_role == body.role {
        return Ok(Json(ApiResponse::ok(format!("Device '{}' already has role '{}'", target.display_name, body.role))));
    }

    sqlx::query("UPDATE device_users SET role = ? WHERE id = ?")
        .bind(body.role.to_string())
        .bind(user_id)
        .execute(&state.db)
        .await?;

    let action = if body.role == Role::Admin { "promote_user" } else { "demote_user" };
    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, ?, ?)")
        .bind(admin.id)
        .bind(action)
        .bind(format!("Changed device '{}' ({}) role from '{}' to '{}'", target.display_name, user_id, old_role, body.role))
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok(format!("Device '{}' role changed to '{}'", target.display_name, body.role))))
}
