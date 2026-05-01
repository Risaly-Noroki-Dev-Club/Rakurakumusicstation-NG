/// 基于 JWT 的身份验证：令牌生成、验证和提取中间件。

use crate::error::AppError;
use crate::models::{Claims, User};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::SqlitePool;
use std::sync::Arc;

/// 为用户生成 JWT 令牌。
pub fn generate_token(user: &User, secret: &str, expiry_hours: u64) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(expiry_hours as i64)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode error: {}", e)))
}

/// 解码并验证 JWT 令牌。
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}

/// 从 Authorization 头中提取 bearer 令牌。
pub fn extract_bearer_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            if value.starts_with("Bearer ") {
                Some(value[7..].to_string())
            } else {
                None
            }
        })
}

/// Axum 的已认证用户提取器。
/// 在处理器参数中用作 AuthUser 来要求认证。
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub role: String,
}

/// 从 JWT 中提取当前用户，从数据库加载完整 User 以检查封禁状态。
/// 如果没有有效令牌或用户被封禁，返回 401。
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 我们需要访问 AppState；这使用了 Axum 的扩展机制。
        // 实际上我们将使用中间件方式替代。目前，定义
        // 此类型用于文档说明，并在处理器中直接提取。
        // 实际提取通过下面的 require_auth() 辅助函数完成。
        let _ = parts;
        let _ = state;
        unreachable!("Use require_auth() helper instead of FromRequestParts")
    }
}

/// 从请求头中提取已认证用户的 claims。不检查数据库封禁状态。
pub fn require_claims(parts: &Parts, secret: &str) -> Result<Claims, AppError> {
    let token = extract_bearer_token(parts)
        .or_else(|| {
            // 也为 WebSocket 升级检查 cookie
            parts
                .headers
                .get(header::COOKIE)
                .and_then(|v| v.to_str().ok())
                .and_then(|cookie_str| {
                    cookie_str.split(';')
                        .find(|c| c.trim().starts_with("token="))
                        .map(|c| c.trim()[6..].to_string())
                })
        })
        .ok_or(AppError::Unauthorized)?;

    validate_token(&token, secret)
}

/// 完整的认证检查：验证 JWT + 从数据库加载用户 + 检查封禁。
pub async fn require_auth(parts: &Parts, db: &SqlitePool, secret: &str) -> Result<AuthUser, AppError> {
    let claims = require_claims(parts, secret)?;

    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(claims.sub.parse::<i64>().unwrap_or(0))
        .fetch_optional(db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if user.is_banned() {
        return Err(AppError::Banned);
    }

    Ok(AuthUser {
        id: user.id,
        username: user.username,
        role: user.role,
    })
}

/// 检查用户是否为管理员。
pub fn require_admin(auth_user: &AuthUser) -> Result<(), AppError> {
    if auth_user.role == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("Admin privileges required".into()))
    }
}

/// 可选认证：如果已认证则返回 Some(AuthUser)，否则返回 None。
/// 在缺失/无效令牌时不会失败 — 仅返回 None。
pub async fn optional_auth(parts: &Parts, db: &SqlitePool, secret: &str) -> Option<AuthUser> {
    match require_claims(parts, secret) {
        Ok(claims) => {
            let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
                .bind(claims.sub.parse::<i64>().unwrap_or(0))
                .fetch_optional(db)
                .await
                .ok()??;

            if user.is_banned() {
                return None;
            }

            Some(AuthUser {
                id: user.id,
                username: user.username,
                role: user.role,
            })
        }
        Err(_) => None,
    }
}

/// 使用 Argon2id 哈希密码。
pub fn hash_password(password: &str) -> Result<String, AppError> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    Ok(hash.to_string())
}

/// 对照 Argon2id 哈希验证密码。
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    use argon2::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid password hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
