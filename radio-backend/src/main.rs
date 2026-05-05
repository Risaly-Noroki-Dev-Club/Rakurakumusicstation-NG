/// Rakuraku Music Station NG - Rust 业务后端
///
/// 处理 HTTP API、WebSocket 广播、设备身份验证（基于 Cookie）、
/// 队列管理、歌词解析，并内嵌音频引擎。

mod auth;
mod config;
mod db;
mod error;
mod lyrics;
mod models;
mod queue_manager;
mod routes;
mod websocket;

use axum::{
    http::{header, HeaderMap, Request},
    middleware::{self, Next},
    response::Response,
    extract::State,
};
use db::AppState;
use radio_engine::config::BUFFER_CAPACITY;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// 中间件：确保每个请求都有一个 device_token Cookie。
async fn device_cookie_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let device_token = auth::extract_device_token(&headers);
    let mut response = next.run(request).await;

    if device_token.is_none() {
        let new_token = auth::generate_device_token();

        if let Err(e) = auth::ensure_device_user(&state.db, &new_token).await {
            tracing::error!("Failed to create device user: {:?}", e);
        }

        let max_age = state.config.device.cookie_max_age_days * 86400;
        let cookie_value = format!(
            "device_token={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            new_token, max_age
        );

        if let Ok(val) = header::HeaderValue::from_str(&cookie_value) {
            response.headers_mut().insert(header::SET_COOKIE, val);
        }
    }

    response
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let config = config::AppConfig::load_default()?;
    tracing::info!(
        "Starting {} on port {}",
        config.station.name,
        config.server.port
    );

    // 初始化音频引擎
    let media_path = config.audio_engine.media_path.clone();
    let ring_buffer = radio_engine::ring_buffer::RingBuffer::new(BUFFER_CAPACITY);
    let (mut player, player_handle) =
        radio_engine::player::Player::new(ring_buffer.clone(), media_path.clone());

    // 初始化播放列表并启动播放器
    player.init_playlist().await;
    tokio::spawn(async move { player.run().await });

    tracing::info!("Audio engine started, media path: {}", media_path);

    // 初始化应用状态
    let state = Arc::new(AppState::new(config, ring_buffer, player_handle).await?);

    // 启动引擎状态轮询器，将播放状态转发给 WebSocket 客户端
    websocket::start_engine_state_poller(state.clone());

    // 构建路由
    let app = routes::build_router(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), device_cookie_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .allow_credentials(true),
        );

    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    tracing::info!("HTTP server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
