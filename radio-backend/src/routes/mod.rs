/// 电台后端 HTTP API 的路由模块。

pub mod auth;
pub mod songs;
pub mod playlist;
pub mod queue;
pub mod admin;
pub mod favorites;
pub mod ncm;

use axum::{Router, routing::{get, post}};
use crate::db::AppState;
use std::sync::Arc;

/// 构建组合的应用程序路由器。
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // WebSocket 端点
        .route("/ws", get(crate::websocket::ws_handler))
        // 认证路由
        .nest("/api/auth", auth::auth_routes())
        // 初始化设置（首次运行）
        .route("/api/setup", post(setup))
        // 歌曲库
        .nest("/api/songs", songs::song_routes())
        // 用户播放列表
        .nest("/api/playlists", playlist::playlist_routes())
        // 共享电台队列
        .nest("/api/queue", queue::queue_routes())
        // 管理端点
        .nest("/api/admin", admin::admin_routes())
        // 用户个人网易云账号
        .nest("/api/ncm", ncm::ncm_routes())
        // 收藏夹
        .nest("/api/favorites", favorites::favorites_routes())
        // 电台信息（公开）
        .route("/api/station", get(station_info))
        // 正在播放（公开）
        .route("/api/now-playing", get(queue::now_playing))
        // 静态文件服务 + SPA 回退（unknown paths → index.html）
        .fallback_service(
            tower_http::services::ServeDir::new("static")
                .not_found_service(tower_http::services::ServeFile::new("static/index.html"))
        )
        .with_state(state)
}

/// GET /api/station — 公开的电台信息
async fn station_info(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    let ws_host = if state.config.server.host == "0.0.0.0" {
        "localhost"
    } else {
        &state.config.server.host
    };

    let has_admin = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM users WHERE role = 'admin'"
    )
    .fetch_one(&state.db)
    .await
    .map(|r| r.0 > 0)
    .unwrap_or(false);

    axum::Json(serde_json::json!({
        "name": state.config.station.name,
        "subtitle": state.config.station.subtitle,
        "primary_color": state.config.station.primary_color,
        "secondary_color": state.config.station.secondary_color,
        "bg_color": state.config.station.bg_color,
        "stream_url": state.config.audio_engine.resolve_stream_url(),
        "ws_url": format!("ws://{}:{}/ws", ws_host, state.config.server.port),
        "needs_setup": !has_admin,
    }))
}

/// POST /api/setup — 首次运行创建管理员账户
async fn setup(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::Json(req): axum::Json<crate::models::RegisterRequest>,
) -> Result<axum::Json<crate::models::ApiResponse<crate::models::AuthResponse>>, crate::error::AppError> {
    use crate::auth;
    use crate::error::AppError;

    // 验证输入
    if req.username.len() < 3 || req.username.len() > 32 {
        return Err(AppError::BadRequest("Username must be 3-32 characters".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("Password must be at least 6 characters".into()));
    }

    // 在事务中检查管理员、检查用户名冲突并插入，避免 TOCTOU 竞态条件
    let mut tx = state.db.begin().await?;

    let has_admin: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE role = 'admin'"
    )
    .fetch_one(&mut *tx)
    .await?;

    if has_admin.0 > 0 {
        return Err(AppError::Forbidden("Setup has already been completed".into()));
    }

    let existing = sqlx::query_as::<_, (i64,)>("SELECT id FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&mut *tx)
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("Username already taken".into()));
    }

    let password_hash = auth::hash_password(&req.password)?;

    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, role) VALUES (?, ?, 'admin')"
    )
    .bind(&req.username)
    .bind(&password_hash)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let user_id = result.last_insert_rowid();

    let user = crate::models::User {
        id: user_id,
        username: req.username.clone(),
        password_hash,
        role: "admin".into(),
        banned_until: None,
        created_at: chrono::Utc::now().naive_utc(),
    };

    let token = auth::generate_token(&user, &state.jwt_secret, state.config.jwt.expiry_hours)?;

    tracing::info!("Initial setup completed: admin user '{}' created", user.username);

    Ok(axum::Json(crate::models::ApiResponse::ok(crate::models::AuthResponse {
        token,
        user: user.into(),
    })))
}
