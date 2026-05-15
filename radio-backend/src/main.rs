#![allow(dead_code)]

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
mod services;
mod websocket;

use axum::{
    http::{header, Method, Request},
    middleware::{self, Next},
    response::Response,
    extract::State,
};
use db::AppState;
use radio_engine::config::BUFFER_CAPACITY;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// 中间件：确保每个请求都有一个 device_token Cookie。
async fn device_cookie_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let is_secure = request_is_secure(request.headers());
    let device_token = auth::extract_device_token(request.headers());
    let new_token = if device_token.is_none() {
        let new_token = auth::generate_device_token();

        if let Err(e) = auth::ensure_device_user(&state.db, &new_token).await {
            tracing::error!("Failed to create device user: {:?}", e);
        }

        let mut cookie_header = request
            .headers()
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        if !cookie_header.is_empty() {
            cookie_header.push_str("; ");
        }
        cookie_header.push_str("device_token=");
        cookie_header.push_str(&new_token);

        if let Ok(val) = header::HeaderValue::from_str(&cookie_header) {
            request.headers_mut().insert(header::COOKIE, val);
        }

        Some(new_token)
    } else {
        None
    };

    let mut response = next.run(request).await;

    if let Some(new_token) = new_token {
        let max_age = state.config.device.cookie_max_age_days * 86400;
        let cookie_path = state.config.server.base_path.as_str();
        let secure_attr = if is_secure { "; Secure" } else { "" };
        let cookie_value = format!(
            "device_token={}; Path={}; HttpOnly; SameSite=Lax; Max-Age={}{}",
            new_token, cookie_path, max_age, secure_attr
        );

        if let Ok(val) = header::HeaderValue::from_str(&cookie_value) {
            response.headers_mut().insert(header::SET_COOKIE, val);
        }
    }

    response
}

fn request_is_secure(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
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
        "Starting {} on port {} with base path {}",
        config.station.name,
        config.server.port,
        config.server.base_path
    );

    // 初始化音频引擎
    let media_path = config.audio_engine.media_path.clone();
    let ring_buffer = radio_engine::ring_buffer::RingBuffer::new(BUFFER_CAPACITY);
    let (mut player, player_handle) =
        radio_engine::player::Player::new(ring_buffer.clone(), media_path.clone());

    // 初始化播放队列并启动播放器
    player.init_play_queue().await;
    tokio::spawn(async move { player.run().await });

    tracing::info!("Audio engine started, media path: {}", media_path);

    // 初始化应用状态
    let state = Arc::new(AppState::new(config, ring_buffer, player_handle).await?);

    // 把 DB 里 status='pending' 的曲子重新装回引擎请求队列（重启续播）。
    if let Err(e) = queue_manager::rehydrate_engine_queue(&state).await {
        tracing::error!("Failed to rehydrate engine queue from DB: {:?}", e);
    }

    // 启动引擎状态轮询器，将播放状态转发给 WebSocket 客户端
    websocket::start_engine_state_poller(state.clone());

    // 构建路由
    let app = routes::build_router(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), device_cookie_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::predicate(|_origin, _parts| true))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::PATCH,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    header::CONTENT_TYPE,
                    header::ACCEPT,
                    header::AUTHORIZATION,
                ])
                .allow_credentials(true),
        );

    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    tracing::info!("HTTP server listening on http://{}", addr);

    let listener = bind_with_keepalive(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Bind a TCP listener with TCP keepalive enabled on the listening socket.
///
/// Why: hyper, when serving an HTTP/1.1 streaming response that hasn't tried
/// to write yet, doesn't actively poll the TCP read side, so an abrupt client
/// disappearance (e.g. `timeout 1 curl`) leaves the server-side socket stuck
/// in ESTABLISHED forever. The tokio task spawned per stream stays parked in
/// `wait_for_data`, the ring-buffer reader is never dropped, and the fd is
/// never released — fd/conn/task accumulate indefinitely under load.
///
/// On Linux SO_KEEPALIVE plus the per-socket TCP_KEEPIDLE/INTVL/CNT options
/// are inherited by accepted sockets, so setting them on the listening socket
/// is enough. With these settings the kernel kills idle dead connections
/// within ~50s, which lets hyper drop the body and our task exit cleanly.
async fn bind_with_keepalive(addr: &str) -> anyhow::Result<tokio::net::TcpListener> {
    use socket2::{Domain, Protocol, Socket, TcpKeepalive, Type};
    use std::net::SocketAddr;
    use std::time::Duration;

    let socket_addr: SocketAddr = addr.parse()?;
    let domain = if socket_addr.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_nonblocking(true)?;
    socket.set_reuse_address(true)?;

    let mut keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(20))
        .with_interval(Duration::from_secs(10));
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        keepalive = keepalive.with_retries(3);
    }
    socket.set_tcp_keepalive(&keepalive)?;

    socket.bind(&socket_addr.into())?;
    socket.listen(1024)?;

    let std_listener: std::net::TcpListener = socket.into();
    let listener = tokio::net::TcpListener::from_std(std_listener)?;
    Ok(listener)
}
