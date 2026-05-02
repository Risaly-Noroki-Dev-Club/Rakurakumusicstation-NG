/// 认证路由：注册、登录、获取当前用户。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{
    ApiResponse, AuthResponse, LoginRequest, RegisterRequest,
};
use axum::{
    extract::State,
    http::{header, HeaderMap},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(get_me))
}

/// POST /api/auth/register
async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, AppError> {
    // 验证输入
    if req.username.len() < 3 || req.username.len() > 32 {
        return Err(AppError::BadRequest("Username must be 3-32 characters".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("Password must be at least 6 characters".into()));
    }

    // 检查重复用户名
    let existing = sqlx::query_as::<_, (i64,)>("SELECT id FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&state.db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Username already taken".into()));
    }

    // 哈希密码
    let password_hash = auth::hash_password(&req.password)?;

    // 插入用户
    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, role) VALUES (?, ?, 'user')"
    )
    .bind(&req.username)
    .bind(&password_hash)
    .execute(&state.db)
    .await?;

    let user_id = result.last_insert_rowid();

    // 生成 JWT
    let user = crate::models::User {
        id: user_id,
        username: req.username.clone(),
        password_hash,
        role: "user".into(),
        banned_until: None,
        created_at: chrono::Utc::now().naive_utc(),
    };

    let token = auth::generate_token(&user, &state.jwt_secret, state.config.jwt.expiry_hours)?;

    Ok(Json(ApiResponse::ok(AuthResponse {
        token,
        user: user.into(),
    })))
}

/// POST /api/auth/login
async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, AppError> {
    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized)?;

    // 验证密码
    if !auth::verify_password(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    // 检查是否被封禁
    if user.is_banned() {
        return Err(AppError::Banned);
    }

    let token = auth::generate_token(&user, &state.jwt_secret, state.config.jwt.expiry_hours)?;

    tracing::info!("User '{}' logged in", user.username);

    Ok(Json(ApiResponse::ok(AuthResponse {
        token,
        user: user.into(),
    })))
}

/// GET /api/auth/me — 从 JWT 获取当前用户信息
async fn get_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<crate::models::UserPublic>>, AppError> {
    let secret = &state.jwt_secret;

    // 从 Authorization 头提取 JWT
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let claims = auth::validate_token(token, secret)?;

    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(claims.sub.parse::<i64>().map_err(|_| AppError::Unauthorized)?)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::ok(user.into())))
}
