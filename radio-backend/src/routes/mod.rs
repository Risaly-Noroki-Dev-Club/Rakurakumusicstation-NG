/// 电台后端 HTTP API 的路由模块。

pub mod auth;
pub mod songs;
pub mod playlist;
pub mod queue;
pub mod admin;
pub mod favorites;
pub mod ncm;

use axum::{Router, routing::get};
use crate::db::AppState;
use std::sync::Arc;

/// 构建组合的应用程序路由器。
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // WebSocket 端点
        .route("/ws", get(crate::websocket::ws_handler))
        // 音频流端点
        .route("/stream", get(stream_handler))
        // 设备认证路由
        .nest("/api/auth", auth::auth_routes())
        // 歌曲库
        .nest("/api/songs", songs::song_routes())
        // 用户播放列表
        .nest("/api/playlists", playlist::playlist_routes())
        // 共享电台队列
        .nest("/api/queue", queue::queue_routes())
        // 管理端点
        .nest("/api/admin", admin::admin_routes())
        // 设备个人网易云账号
        .nest("/api/ncm", ncm::ncm_routes())
        // 收藏夹
        .nest("/api/favorites", favorites::favorites_routes())
        // 电台信息（公开）
        .route("/api/station", get(station_info))
        // 正在播放（公开）
        .route("/api/now-playing", get(queue::now_playing))
        // 静态文件服务 + SPA 回退
        .fallback_service(
            tower_http::services::ServeDir::new("static")
                .not_found_service(tower_http::services::ServeFile::new("static/index.html"))
        )
        .with_state(state)
}

/// GET /stream — 音频流端点，从环形缓冲区广播音频数据
async fn stream_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Response {
    use radio_engine::config::AUDIO_CHUNK_SIZE;

    let (tx, response) = radio_engine::stream::create_stream_response();
    let buffer = state.ring_buffer.clone();

    tokio::spawn(async move {
        let reader = buffer.create_reader();

        let mut buf = vec![0u8; AUDIO_CHUNK_SIZE];
        loop {
            let available = reader.wait_for_data(100);
            if available == 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                continue;
            }

            let to_read = std::cmp::min(buf.len(), available);
            let n = reader.read(&mut buf[..to_read]);
            if n > 0 {
                if tx.send(bytes::Bytes::copy_from_slice(&buf[..n])).is_err() {
                    break;
                }
            }
        }
    });

    response
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
        "SELECT COUNT(*) FROM device_users WHERE role = 'admin'"
    )
    .fetch_one(&state.db)
    .await
    .map(|r| r.0 > 0)
    .unwrap_or(false);

    let station = state.station.read().unwrap_or_else(|e| e.into_inner());

    axum::Json(serde_json::json!({
        "name": station.name,
        "subtitle": station.subtitle,
        "primary_color": station.primary_color,
        "secondary_color": station.secondary_color,
        "bg_color": station.bg_color,
        "stream_url": state.config.audio_engine.resolve_stream_url(),
        "ws_url": format!("ws://{}:{}/ws", ws_host, state.config.server.port),
        "needs_setup": !has_admin,
    }))
}
