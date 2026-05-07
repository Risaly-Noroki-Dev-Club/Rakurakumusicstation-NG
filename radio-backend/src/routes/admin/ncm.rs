/// 网易云账号设置路由。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::routes::admin::get_admin;
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

fn music_dl_path() -> std::path::PathBuf {
    std::env::var("MUSIC_DL_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("music_dl.py"))
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

    let result = std::process::Command::new("python3")
        .arg(music_dl_path())
        .arg("--verify-login")
        .arg("--settings")
        .arg(ncm_secrets_path())
        .output();

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
