/// Rakuraku Music Station NG - Rust 业务后端
///
/// 处理 HTTP API、WebSocket 广播、用户认证、队列管理、
/// 歌词解析，以及通过 Redis pub/sub 与 C++ 音频引擎
/// 进行服务间通信。

mod auth;
mod config;
mod db;
mod error;
mod lyrics;
mod models;
mod queue_manager;
mod routes;
mod websocket;

use db::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化 tracing/日志记录
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    // 加载配置
    let config = config::AppConfig::load_default()?;
    tracing::info!(
        "Starting {} on port {}",
        config.station.name,
        config.server.port
    );

    // 初始化应用状态（数据库、Redis、WebSocket 通道）
    let state = Arc::new(AppState::new(config).await?);

    // 启动 Redis 订阅者，将 playback_state 转发给 WebSocket 客户端
    websocket::start_redis_subscriber(state.clone()).await;

    // 构建路由
    let app = routes::build_router(state.clone())
        .layer(
            // 配置 CORS，允许来自任何来源的浏览器客户端
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // 绑定并启动服务
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    tracing::info!("HTTP server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
