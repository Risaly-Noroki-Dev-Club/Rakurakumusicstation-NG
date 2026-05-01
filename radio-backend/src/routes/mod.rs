/// 电台后端 HTTP API 的路由模块。

pub mod auth;
pub mod songs;
pub mod playlist;
pub mod queue;
pub mod admin;
pub mod favorites;

use axum::{Router, routing::get};
use crate::db::AppState;
use std::sync::Arc;

/// 构建组合的应用程序路由器。
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // WebSocket 端点
        .route("/ws", get(crate::websocket::ws_handler))
        // 认证路由
        .nest("/api/auth", auth::auth_routes())
        // 歌曲库
        .nest("/api/songs", songs::song_routes())
        // 用户播放列表
        .nest("/api/playlists", playlist::playlist_routes())
        // 共享电台队列
        .nest("/api/queue", queue::queue_routes())
        // 管理端点
        .nest("/api/admin", admin::admin_routes())
        // 收藏夹
        .nest("/api/favorites", favorites::favorites_routes())
        // 电台信息（公开）
        .route("/api/station", get(station_info))
        // 正在播放（公开）
        .route("/api/now-playing", get(queue::now_playing))
        .with_state(state)
}

/// GET /api/station — 公开的电台信息
async fn station_info(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "name": state.config.station.name,
        "subtitle": state.config.station.subtitle,
        "primary_color": state.config.station.primary_color,
        "secondary_color": state.config.station.secondary_color,
        "bg_color": state.config.station.bg_color,
        "stream_url": format!("{}:{}/stream", state.config.audio_engine.base_url, 2240),
        "ws_url": format!("ws://{}:{}/ws", state.config.server.host, state.config.server.port),
    }))
}
