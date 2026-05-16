pub mod batch_download;
pub mod download;
pub mod logout;
pub mod ncm;
pub mod playback;
pub mod settings;
pub mod songs;
pub mod stats;
pub mod upload;
/// 管理员路由入口模块。
pub mod users;

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use axum::{extract::DefaultBodyLimit, http::HeaderMap, Router};
use std::sync::Arc;

/// 构建管理员路由。
pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        // 设备用户管理
        .route("/users", axum::routing::get(users::list_users))
        .route("/users/:id/ban", axum::routing::post(users::ban_user))
        .route("/users/:id/unban", axum::routing::post(users::unban_user))
        .route("/users/:id/role", axum::routing::put(users::set_user_role))
        // 统计与日志
        .route("/stats", axum::routing::get(stats::stats))
        .route("/logs", axum::routing::get(stats::get_logs))
        // 歌曲管理
        .route("/rescan-songs", axum::routing::post(songs::rescan_songs))
        .route("/songs", axum::routing::get(songs::list_all_songs))
        .route("/songs/:id", axum::routing::delete(songs::delete_song))
        // 上传 (带 100MB body limit)
        .nest(
            "/upload",
            Router::new()
                .route("/", axum::routing::post(upload::upload_song))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // 系统设置
        .route(
            "/settings",
            axum::routing::get(settings::get_settings).post(settings::save_settings),
        )
        .nest(
            "/settings/icon",
            Router::new()
                .route("/", axum::routing::post(settings::upload_icon))
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024)),
        )
        // 播放控制
        .route("/playlist/next", axum::routing::post(playback::skip_next))
        .route("/playlist/prev", axum::routing::post(playback::skip_prev))
        // 批量下载
        .route("/download", axum::routing::post(download::start_download))
        .route(
            "/download/stream",
            axum::routing::get(download::download_stream),
        )
        .route(
            "/download/status",
            axum::routing::get(download::download_status),
        )
        // 批量下载（新版）
        .route(
            "/download/batch",
            axum::routing::post(batch_download::start_batch_download),
        )
        .route(
            "/download/batch/stream",
            axum::routing::get(batch_download::batch_download_stream),
        )
        .route(
            "/download/batch/status",
            axum::routing::get(batch_download::batch_download_status),
        )
        // 网易云账号
        .route(
            "/ncm",
            axum::routing::get(ncm::get_ncm_settings).post(ncm::save_ncm_settings),
        )
        .route("/ncm/test", axum::routing::post(ncm::test_ncm_login))
        .route("/ncm/playlist", axum::routing::post(ncm::import_playlist))
        .route("/ncm/import", axum::routing::post(ncm::start_ncm_import))
        // 管理退出
        .route("/logout", axum::routing::post(logout::logout))
}

/// 从请求头中提取已认证管理员设备用户。
pub async fn get_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::auth::AuthUser, AppError> {
    auth::require_admin_from_headers(headers, &state.db).await
}
