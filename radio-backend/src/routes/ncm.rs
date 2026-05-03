/// 用户个人网易云账号路由：设置、查看、测试个人网易云凭据。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, NcmStatus, SaveNcmRequest};
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
fn user_ncm_secrets(user: &crate::models::UserNcm) -> serde_json::Value {
    let mut secrets = serde_json::json!({
        "ncm_cookie": user.ncm_cookie,
        "ncm_phone": user.ncm_phone,
        "ncm_password": user.ncm_password,
    });
    // 清理空值
    if let Some(map) = secrets.as_object_mut() {
        map.retain(|_, v| !v.as_str().map(|s| s.is_empty()).unwrap_or(false));
    }
    secrets
}

/// GET /api/ncm — 获取当前用户的网易云账号状态
pub async fn get_ncm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<NcmStatus>>, AppError> {
    let user = auth::require_auth_from_headers(&headers, &state.db, &state.jwt_secret).await?;

    let ncm = sqlx::query_as::<_, crate::models::UserNcm>(
        "SELECT * FROM user_ncm WHERE user_id = ?"
    )
    .bind(user.id)
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

/// POST /api/ncm — 保存当前用户的网易云账号设置
pub async fn save_ncm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SaveNcmRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user = auth::require_auth_from_headers(&headers, &state.db, &state.jwt_secret).await?;

    // 查询已有记录
    let existing = sqlx::query_as::<_, crate::models::UserNcm>(
        "SELECT * FROM user_ncm WHERE user_id = ?"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;

    let ncm_cookie = body.cookie.unwrap_or_default().trim().to_string();
    let ncm_phone = body.phone.unwrap_or_default().trim().to_string();
    let ncm_password = body.password.unwrap_or_default().trim().to_string();

    if let Some(record) = existing {
        // 更新：只更新非空字段
        let cookie = if !ncm_cookie.is_empty() { &ncm_cookie } else { &record.ncm_cookie };
        let phone = if !ncm_phone.is_empty() { &ncm_phone } else { &record.ncm_phone };
        let pwd = if !ncm_password.is_empty() { &ncm_password } else { &record.ncm_password };

        sqlx::query(
            "UPDATE user_ncm SET ncm_cookie = ?, ncm_phone = ?, ncm_password = ?, updated_at = datetime('now') WHERE user_id = ?"
        )
        .bind(cookie)
        .bind(phone)
        .bind(pwd)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    } else {
        // 插入
        sqlx::query(
            "INSERT INTO user_ncm (user_id, ncm_cookie, ncm_phone, ncm_password) VALUES (?, ?, ?, ?)"
        )
        .bind(user.id)
        .bind(&ncm_cookie)
        .bind(&ncm_phone)
        .bind(&ncm_password)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(ApiResponse::ok("保存成功".into())))
}

/// POST /api/ncm/test — 测试当前用户的网易云登录
pub async fn test_ncm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user = auth::require_auth_from_headers(&headers, &state.db, &state.jwt_secret).await?;

    let ncm = sqlx::query_as::<_, crate::models::UserNcm>(
        "SELECT * FROM user_ncm WHERE user_id = ?"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::BadRequest("请先配置网易云账号".into()))?;

    if ncm.ncm_cookie.is_empty() && ncm.ncm_phone.is_empty() {
        return Err(AppError::BadRequest("请先配置网易云账号".into()));
    }

    // 写入临时 secrets.json
    let secrets = user_ncm_secrets(&ncm);
    let tmp_path: std::path::PathBuf = std::env::temp_dir().join(format!("radio_ncm_test_{}.json", user.id));
    let content = serde_json::to_string(&secrets)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("序列化失败: {}", e)))?;
    std::fs::write(&tmp_path, content)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("写入失败: {}", e)))?;

    let dl_path = std::env::var("MUSIC_DL_PATH")
        .unwrap_or_else(|_| "music_dl.py".to_string());

    let result = std::process::Command::new("python3")
        .arg(&dl_path)
        .arg("--verify-login")
        .arg("--settings")
        .arg(&tmp_path)
        .output();

    // 清理
    std::fs::remove_file(&tmp_path).ok();

    match result {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let combined = if stderr.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                format!("{}\n{}", stdout.trim(), stderr.trim())
            };
            let success = output.status.success();
            Ok(Json(ApiResponse::ok(serde_json::json!({
                "success": success,
                "output": if combined.is_empty() {
                    if success { "登录成功".to_string() } else { "登录失败".to_string() }
                } else { combined },
            }))))
        }
        Err(e) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": false,
            "output": format!("执行失败: {}", e),
        })))),
    }
}
