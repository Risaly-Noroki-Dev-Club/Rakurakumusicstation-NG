use crate::app::state::AppState;
/// 设备认证路由：获取当前设备信息、设置显示名称、申请管理员。
use crate::auth;
use crate::error::AppError;
use crate::models::{ApiResponse, ClaimAdminRequest, SetDisplayNameRequest};
use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(get_me))
        .route("/name", post(set_display_name))
        .route("/claim-admin", post(claim_admin))
}

/// GET /api/auth/me — 获取当前设备信息
async fn get_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user = auth::lookup_device_auth(&headers, &state.db).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "id": user.id,
        "display_name": user.display_name,
        "role": user.role,
    }))))
}

/// POST /api/auth/name — 设置当前设备的显示名称
async fn set_display_name(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<SetDisplayNameRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = auth::require_device_auth(&headers, &state.db).await?;

    let name = req.display_name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Display name cannot be empty".into()));
    }
    if name.len() > 32 {
        return Err(AppError::BadRequest(
            "Display name must be 32 characters or less".into(),
        ));
    }

    sqlx::query("UPDATE device_users SET display_name = ? WHERE id = ?")
        .bind(name)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok(format!(
        "Display name set to '{}'",
        name
    ))))
}

/// POST /api/auth/claim-admin — 使用管理员设置令牌升级为管理员
async fn claim_admin(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ClaimAdminRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let device_token = auth::extract_device_token(&headers).ok_or(AppError::Unauthorized)?;

    auth::claim_admin(
        &state.db,
        &device_token,
        &req.admin_setup_token,
        &state.config.device.admin_setup_token,
    )
    .await?;

    Ok(Json(ApiResponse::ok("Admin privileges granted".into())))
}
