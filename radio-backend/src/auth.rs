/// 基于设备的身份验证：通过 httpOnly Cookie 中的 device_token 识别设备。
/// 兼容 WebSocket：支持从 Cookie 头或查询参数读取 device_token。
use crate::error::AppError;
use crate::models::DeviceUser;
use axum::http::{header, HeaderMap};
use sqlx::SqlitePool;

/// Axum 的已认证设备用户提取器。
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub display_name: String,
    pub role: String,
    pub device_token: String,
}

/// 检查用户是否为管理员。
pub fn require_admin(auth_user: &AuthUser) -> Result<(), AppError> {
    if auth_user.role == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("Admin privileges required".into()))
    }
}

/// 生成一个随机的设备令牌（使用 UUID v4）。
pub fn generate_device_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 从 Cookie 头提取 device_token 值。
pub fn extract_device_token_from_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|pair| {
                let pair = pair.trim();
                pair.strip_prefix("device_token=").map(|v| v.to_string())
            })
        })
}

/// 从查询参数提取 device_token（WebSocket 回退）。
pub fn extract_device_token_from_query(headers: &HeaderMap) -> Option<String> {
    // Axum 将 WebSocket 查询参数放在 Sec-WebSocket-Protocol 或自定义头中未直接暴露。
    // 最可靠的方案是在 WebSocket 升级 URL 中解析查询字符串。
    // 这里从前端传入的 custom header 作为回退。
    headers
        .get("x-device-token")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
}

/// 从请求中提取 device_token（优先 Cookie，回退到 header）。
pub fn extract_device_token(headers: &HeaderMap) -> Option<String> {
    extract_device_token_from_cookie(headers).or_else(|| extract_device_token_from_query(headers))
}

/// 通过 device_token 查找或创建设备用户。
/// 新设备自动以默认显示名称 "Listener-XXXX" 和 "user" 角色创建。
pub async fn ensure_device_user(db: &SqlitePool, device_token: &str) -> Result<AuthUser, AppError> {
    let user = sqlx::query_as::<_, DeviceUser>("SELECT * FROM device_users WHERE device_token = ?")
        .bind(device_token)
        .fetch_optional(db)
        .await?;

    match user {
        Some(u) => {
            if u.is_banned() {
                return Err(AppError::Banned);
            }
            Ok(AuthUser {
                id: u.id,
                display_name: u.display_name.clone(),
                role: u.role.clone(),
                device_token: u.device_token.clone(),
            })
        }
        None => {
            // 创建新设备用户
            let result = sqlx::query(
                "INSERT INTO device_users (device_token, display_name, role) VALUES (?, '', 'user')"
            )
            .bind(device_token)
            .execute(db)
            .await?;

            let id = result.last_insert_rowid();
            let display_name = DeviceUser::default_display_name(id);

            sqlx::query("UPDATE device_users SET display_name = ? WHERE id = ?")
                .bind(&display_name)
                .bind(id)
                .execute(db)
                .await?;

            Ok(AuthUser {
                id,
                display_name,
                role: "user".into(),
                device_token: device_token.to_string(),
            })
        }
    }
}

/// 从 HeaderMap 认证设备用户（完整认证，用于受保护路由）。
pub async fn require_device_auth(
    headers: &HeaderMap,
    db: &SqlitePool,
) -> Result<AuthUser, AppError> {
    let device_token = extract_device_token(headers).ok_or(AppError::Unauthorized)?;
    ensure_device_user(db, &device_token).await
}

/// 从 HeaderMap 认证管理员设备用户。
pub async fn require_admin_from_headers(
    headers: &HeaderMap,
    db: &SqlitePool,
) -> Result<AuthUser, AppError> {
    let user = require_device_auth(headers, db).await?;
    require_admin(&user)?;
    Ok(user)
}

/// 可选认证（已登录返回 Some，访客返回 None）。
pub async fn optional_device_auth(headers: &HeaderMap, db: &SqlitePool) -> Option<AuthUser> {
    let device_token = extract_device_token(headers)?;
    ensure_device_user(db, &device_token).await.ok()
}

/// 验证 admin_setup_token 并将当前设备升级为管理员。
pub async fn claim_admin(
    db: &SqlitePool,
    device_token: &str,
    setup_token: &str,
    configured_token: &str,
) -> Result<AuthUser, AppError> {
    if configured_token.is_empty() {
        return Err(AppError::Forbidden(
            "Admin setup is disabled: no admin_setup_token configured".into(),
        ));
    }
    if setup_token != configured_token {
        return Err(AppError::Forbidden("Invalid admin setup token".into()));
    }

    let user = sqlx::query_as::<_, DeviceUser>("SELECT * FROM device_users WHERE device_token = ?")
        .bind(device_token)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::Unauthorized)?;

    sqlx::query("UPDATE device_users SET role = 'admin' WHERE id = ?")
        .bind(user.id)
        .execute(db)
        .await?;

    Ok(AuthUser {
        id: user.id,
        display_name: user.display_name,
        role: "admin".into(),
        device_token: user.device_token,
    })
}
