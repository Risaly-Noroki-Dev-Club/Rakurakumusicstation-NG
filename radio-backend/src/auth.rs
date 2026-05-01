/// 基于 JWT 的身份验证：令牌生成、验证和提取中间件。

use crate::error::AppError;
use crate::models::{Claims, User};
use axum::{
    http::{header, HeaderMap},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::SqlitePool;

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

/// Axum 的已认证用户提取器。
/// 在处理器参数中用作 AuthUser 来要求认证。
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub role: String,
}

/// 检查用户是否为管理员。
pub fn require_admin(auth_user: &AuthUser) -> Result<(), AppError> {
    if auth_user.role == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("Admin privileges required".into()))
    }
}

/// 从 HeaderMap 提取 Bearer token 并完整认证（供路由处理器使用）。
pub async fn require_auth_from_headers(
    headers: &HeaderMap,
    db: &SqlitePool,
    secret: &str,
) -> Result<AuthUser, AppError> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let claims = validate_token(token, secret)?;

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

/// 从 HeaderMap 提取并验证管理员用户。
pub async fn require_admin_from_headers(
    headers: &HeaderMap,
    db: &SqlitePool,
    secret: &str,
) -> Result<AuthUser, AppError> {
    let user = require_auth_from_headers(headers, db, secret).await?;
    require_admin(&user)?;
    Ok(user)
}

/// 从 HeaderMap 可选认证（已登录返回 Some，访客返回 None）。
pub async fn optional_auth_from_headers(
    headers: &HeaderMap,
    db: &SqlitePool,
    secret: &str,
) -> Option<AuthUser> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))?;

    let claims = validate_token(token, secret).ok()?;

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
