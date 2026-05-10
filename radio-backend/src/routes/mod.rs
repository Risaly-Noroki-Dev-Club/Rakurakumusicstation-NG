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
///
/// ## 内存与连接安全
/// - 使用 bounded channel + `send_timeout` 主动检测死连接：当 hyper 因 TCP
///   连接已断但还没察觉而停止消费时，channel 满后 send 会阻塞，超时即视为
///   死客户端并退出循环（修复 hyper 在 streaming 响应空闲期间漏检客户端断
///   开导致 tokio task / fd / reader 大量泄漏的问题）。
/// - reader 在 task 退出时 Drop 自动清理 ring buffer 位置。
async fn stream_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Response {
    use radio_engine::config::AUDIO_CHUNK_SIZE;
    use std::time::{Duration, Instant};

    // 单次 send 的超时上限：channel 满且对端在此时间内未消费即判定连接死亡
    const SEND_TIMEOUT: Duration = Duration::from_secs(5);
    // 空闲超时：长时间没有成功推送数据则主动断开（防止极端情况下任务永不退出）
    const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
    // 单次等待数据的最大时间，到期后回到循环顶部重新检查 tx 状态
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
                Ok(Err(_)) => break, // 接收端已 drop（hyper 已检测到断开）
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
