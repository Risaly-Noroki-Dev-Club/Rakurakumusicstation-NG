/// 退出登录路由。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

/// POST /api/admin/logout — 退出登录（前端清除 token 即可）
pub async fn logout(
    State(_state): State<Arc<AppState>>,
    _headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::ok("已退出".into())))
}
