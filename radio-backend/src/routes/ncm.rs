/// 设备个人网易云账号路由：设置、查看、测试个人网易云凭据。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, NcmStatus, SaveNcmRequest};
use crate::services::ncm::NcmClient;
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

/// 临时 secrets.json 路径（用于测试登录时传递给 music_dl.py）
fn user_ncm_secrets(ncm: &crate::models::UserNcm) -> serde_json::Value {
    let mut secrets = serde_json::json!({
        "ncm_cookie": ncm.ncm_cookie,
        "ncm_phone": ncm.ncm_phone,
        "ncm_password": ncm.ncm_password,
    });
    if let Some(map) = secrets.as_object_mut() {
        map.retain(|_, v| !v.as_str().map(|s| s.is_empty()).unwrap_or(false));
    }
    secrets
}

fn extract_music_u(ncm: &crate::models::UserNcm) -> Option<String> {
    if !ncm.ncm_cookie.is_empty() {
        for part in ncm.ncm_cookie.split(';') {
            let part = part.trim();
            if part.starts_with("MUSIC_U=") {
                return Some(part.strip_prefix("MUSIC_U=").unwrap_or("").to_string());
            }
        }
    }
    if !ncm.ncm_phone.is_empty() {
        return Some(String::new());
    }
    None
}

/// GET /api/ncm — 获取当前设备的网易云账号状态
pub async fn get_ncm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<NcmStatus>>, AppError> {
    let device = auth::require_device_auth(&headers, &state.db).await?;

    let ncm = sqlx::query_as::<_, crate::models::UserNcm>(
        "SELECT * FROM user_ncm WHERE device_user_id = ?"
    )
    .bind(device.id)
    .fetch_optional(&state.db)
    .await?;

    match ncm {
        Some(record) => {
            let configured = !record.ncm_cookie.is_empty() || !record.ncm_phone.is_empty();
            let method = if !record.ncm_cookie.is_empty() {
                "cookie"
            } else if configured { "phone" } else { "none" };
            let phone = &record.ncm_phone;
            Ok(Json(ApiResponse::ok(NcmStatus {
                configured,
                method: method.to_string(),
                phone_hint: if phone.len() > 4 {
                    format!("{}...{}", &phone[..3], &phone[phone.len()-2..])
                } else { phone.clone() },
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
        "SELECT * FROM user_ncm WHERE device_user_id = ?"
    )
    .bind(device.id)
    .fetch_optional(&state.db)
    .await?;

    let ncm_cookie = body.cookie.unwrap_or_default().trim().to_string();
    let ncm_phone = body.phone.unwrap_or_default().trim().to_string();
    let ncm_password = body.password.unwrap_or_default().trim().to_string();

    if let Some(record) = existing {
        let cookie = if !ncm_cookie.is_empty() { &ncm_cookie } else { &record.ncm_cookie };
        let phone = if !ncm_phone.is_empty() { &ncm_phone } else { &record.ncm_phone };
        let pwd = if !ncm_password.is_empty() { &ncm_password } else { &record.ncm_password };

        sqlx::query(
            "UPDATE user_ncm SET ncm_cookie = ?, ncm_phone = ?, ncm_password = ?, updated_at = datetime('now') WHERE device_user_id = ?"
        )
        .bind(cookie)
        .bind(phone)
        .bind(pwd)
        .bind(device.id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO user_ncm (device_user_id, ncm_cookie, ncm_phone, ncm_password) VALUES (?, ?, ?, ?)"
        )
        .bind(device.id)
        .bind(&ncm_cookie)
        .bind(&ncm_phone)
        .bind(&ncm_password)
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
        "SELECT * FROM user_ncm WHERE device_user_id = ?"
    )
    .bind(device.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::BadRequest("请先配置网易云账号".into()))?;

    if ncm.ncm_cookie.is_empty() && ncm.ncm_phone.is_empty() {
        return Err(AppError::BadRequest("请先配置网易云账号".into()));
    }

    let music_u = match extract_music_u(&ncm) {
        Some(mu) => mu,
        None => {
            return Ok(Json(ApiResponse::ok(serde_json::json!({
                "success": false,
                "output": "无法提取有效的登录凭据",
            }))));
        }
    };

    let client = NcmClient::new(None, Some(music_u));

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
