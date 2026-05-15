/// 电台后端 HTTP API 的路由模块。

pub mod auth;
pub mod songs;
pub mod playlist;
pub mod queue;
pub mod admin;
pub mod favorites;
pub mod ncm;

use crate::db::AppState;
use axum::{
    Json,
    Router,
    http::StatusCode,
    routing::get,
};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

/// 构建组合的应用程序路由器。
pub fn build_router(state: Arc<AppState>) -> Router {
    let api_routes = Router::new()
        .nest("/auth", auth::auth_routes())
        .nest("/songs", songs::song_routes())
        .nest("/playlists", playlist::playlist_routes())
        .nest("/queue", queue::queue_routes())
        .nest("/admin", admin::admin_routes())
        .nest("/ncm", ncm::ncm_routes())
        .nest("/favorites", favorites::favorites_routes())
        .route("/station", get(station_info))
        .route("/now-playing", get(queue::now_playing))
        .fallback(api_not_found);

    Router::new()
        .route("/ws", get(crate::websocket::ws_handler))
        .route("/stream", get(stream_handler))
        .nest("/api", api_routes)
        .with_state(state)
        .fallback_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")))
}

async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "success": false,
            "error": "API endpoint not found",
        })),
    )
}

/// GET /stream — 音频流端点，从环形缓冲区广播音频数据
async fn stream_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Response {
    use radio_engine::config::AUDIO_CHUNK_SIZE;
    use std::time::{Duration, Instant};

    const SEND_TIMEOUT: Duration = Duration::from_secs(5);
    const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
    const WAIT_DATA_MS: u64 = 500;

    let (tx, response) = radio_engine::stream::create_stream_response();
    let buffer = state.ring_buffer.clone();

    tokio::spawn(async move {
        let reader = buffer.create_reader();
        let mut buf = vec![0u8; AUDIO_CHUNK_SIZE];
        let mut last_progress = Instant::now();

        loop {
            if tx.is_closed() {
                break;
            }
            if last_progress.elapsed() > IDLE_TIMEOUT {
                tracing::debug!("Stream idle timeout, closing");
                break;
            }

            let available = reader.wait_for_data(WAIT_DATA_MS).await;
            if available == 0 {
                continue;
            }

            let to_read = std::cmp::min(buf.len(), available);
            let n = reader.read(&mut buf[..to_read]);
            if n == 0 {
                continue;
            }

            let chunk = bytes::Bytes::copy_from_slice(&buf[..n]);
            match tokio::time::timeout(SEND_TIMEOUT, tx.send(chunk)).await {
                Ok(Ok(())) => {
                    last_progress = Instant::now();
                }
                Ok(Err(_)) => break,
                Err(_) => {
                    tracing::debug!("Stream send timeout — client likely dead");
                    break;
                }
            }
        }

        tracing::debug!("Stream client disconnected, reader cleaned up");
    });

    response
}

/// GET /api/station — 公开的电台信息
async fn station_info(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
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
        "stream_url": state.config.audio_engine.resolve_stream_url(Some(&headers), state.config.server.port),
        "ws_url": format!("ws://{}:{}/ws", ws_host, state.config.server.port),
        "needs_setup": !has_admin,
    }))
}
