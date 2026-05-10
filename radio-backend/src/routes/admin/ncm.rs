/// 网易云账号设置路由 — 原生 Rust 实现。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::routes::admin::get_admin;
use crate::services::ncm::NcmClient;
use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

fn ncm_secrets_path() -> std::path::PathBuf {
    std::env::var("NCM_SECRETS_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("secrets.json"))
}

fn read_music_u_from_secrets() -> Option<String> {
    let path = ncm_secrets_path();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    if let Some(cookie) = json.get("ncm_cookie").and_then(|v| v.as_str()) {
        for part in cookie.split(';') {
            let part = part.trim();
            if part.starts_with("MUSIC_U=") {
                return Some(part.strip_prefix("MUSIC_U=").unwrap_or("").to_string());
            }
        }
    }

    if json
        .get("ncm_phone")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false)
    {
        return Some(String::new());
    }

    None
}

/// GET /api/admin/ncm — 获取网易云账号状态
pub async fn get_ncm_settings(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let path = ncm_secrets_path();
    if !path.exists() {
        return Ok(Json(ApiResponse::ok(serde_json::json!({
            "configured": false,
            "method": "none",
            "phone_hint": ""
        }))));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("无法读取 secrets.json")))?;

    let secrets: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or(serde_json::Value::Null);

    let configured = secrets.get("ncm_phone").or(secrets.get("ncm_cookie"))
        .map(|v| !v.as_str().unwrap_or("").is_empty())
        .unwrap_or(false);

    let method = if secrets.get("ncm_cookie")
        .map(|v| !v.as_str().unwrap_or("").is_empty())
        .unwrap_or(false)
    {
        "cookie"
    } else if configured { "phone" } else { "none" };

    let phone = secrets.get("ncm_phone")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "configured": configured,
        "method": method,
        "phone_hint": if phone.len() > 4 {
            format!("{}...{}", &phone[..3], &phone[phone.len()-2..])
        } else { phone.to_string() }
    }))))
}

/// POST /api/admin/ncm — 保存网易云账号设置
pub async fn save_ncm_settings(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let path = ncm_secrets_path();
    let mut secrets: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(map) = secrets.as_object_mut() {
        if let Some(cookie) = body.get("cookie").and_then(|v| v.as_str()) {
            if !cookie.is_empty() {
                map.insert("ncm_cookie".into(), serde_json::Value::String(cookie.to_string()));
            } else {
                map.remove("ncm_cookie");
            }
        }
        if let Some(phone) = body.get("phone").and_then(|v| v.as_str()) {
            if !phone.is_empty() {
                map.insert("ncm_phone".into(), serde_json::Value::String(phone.to_string()));
            } else {
                map.remove("ncm_phone");
            }
        }
        if let Some(password) = body.get("password").and_then(|v| v.as_str()) {
            if !password.is_empty() {
                map.insert("ncm_password".into(), serde_json::Value::String(password.to_string()));
            } else {
                map.remove("ncm_password");
            }
        }
    }

    let content = serde_json::to_string_pretty(&secrets)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("序列化失败: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("写入失败: {}", e)))?;

    Ok(Json(ApiResponse::ok("保存成功".into())))
}

/// POST /api/admin/ncm/test — 测试网易云登录
pub async fn test_ncm_login(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let music_u = match read_music_u_from_secrets() {
        Some(mu) => mu,
        None => {
            return Ok(Json(ApiResponse::ok(serde_json::json!({
                "success": false,
                "output": "未配置网易云账号",
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
