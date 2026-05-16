/// 设备个人网易云账号路由：设置、查看、测试个人网易云凭据。
use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, NcmStatus, SaveNcmRequest};
use crate::services::ncm::{cookie, NcmClient};
use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn ncm_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_ncm).post(save_ncm))
        .route("/test", post(test_ncm))
}

/// GET /api/ncm — 获取当前设备的网易云账号状态
pub async fn get_ncm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<NcmStatus>>, AppError> {
    let device = auth::require_device_auth(&headers, &state.db).await?;

    let ncm = sqlx::query_as::<_, crate::models::UserNcm>(
        "SELECT * FROM user_ncm WHERE device_user_id = ?",
    )
    .bind(device.id)
    .fetch_optional(&state.db)
    .await?;

    match ncm {
        Some(record) => {
            let configured = cookie::has_cookie(&record.ncm_cookie, "MUSIC_U");
            let method = if configured { "cookie" } else { "none" };
            Ok(Json(ApiResponse::ok(NcmStatus {
                configured,
                method: method.to_string(),
                phone_hint: String::new(),
            })))
        }
        None => Ok(Json(ApiResponse::ok(NcmStatus {
            configured: false,
            method: "none".into(),
            phone_hint: String::new(),
        }))),
    }
}

/// POST /api/ncm — 保存当前设备的网易云账号设置
pub async fn save_ncm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SaveNcmRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let device = auth::require_device_auth(&headers, &state.db).await?;

    let existing = sqlx::query_as::<_, crate::models::UserNcm>(
        "SELECT * FROM user_ncm WHERE device_user_id = ?",
    )
    .bind(device.id)
    .fetch_optional(&state.db)
    .await?;

    let ncm_cookie = cookie::validate_login_cookie(&body.cookie.unwrap_or_default())
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if existing.is_some() {
        sqlx::query(
            "UPDATE user_ncm SET ncm_cookie = ?, ncm_phone = ?, ncm_password = ?, updated_at = datetime('now') WHERE device_user_id = ?"
        )
        .bind(&ncm_cookie)
        .bind("")
        .bind("")
        .bind(device.id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO user_ncm (device_user_id, ncm_cookie, ncm_phone, ncm_password) VALUES (?, ?, ?, ?)"
        )
        .bind(device.id)
        .bind(&ncm_cookie)
        .bind("")
        .bind("")
        .execute(&state.db)
        .await?;
    }

    Ok(Json(ApiResponse::ok("保存成功".into())))
}

/// POST /api/ncm/test — 测试当前设备的网易云登录
pub async fn test_ncm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let device = auth::require_device_auth(&headers, &state.db).await?;

    let ncm = sqlx::query_as::<_, crate::models::UserNcm>(
        "SELECT * FROM user_ncm WHERE device_user_id = ?",
    )
    .bind(device.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::BadRequest("请先配置网易云账号".into()))?;

    if !cookie::has_cookie(&ncm.ncm_cookie, "MUSIC_U") {
        return Err(AppError::BadRequest("请先配置网易云账号".into()));
    }

    let client = NcmClient::new(None, Some(ncm.ncm_cookie));

    match client.test_login().await {
        Ok(true) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": true,
            "output": "登录成功",
        })))),
        Ok(false) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": false,
            "output": "登录失败，Cookie 可能已过期",
        })))),
        Err(e) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": false,
            "output": format!("请求失败: {}", e),
        })))),
    }
}
