/// 播放控制路由。
use crate::db::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::routes::admin::get_admin;
use crate::websocket;
use axum::{extract::State, http::HeaderMap, Json};
use std::sync::Arc;

/// POST /api/admin/playlist/next — 切到下一首
pub async fn skip_next(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let cmd = radio_engine::types::AudioCommand {
        cmd_type: radio_engine::types::AudioCommandType::Skip,
        song_id: None,
        file_path: None,
    };

    websocket::publish_command(&state, &cmd).await?;

    Ok(Json(ApiResponse::ok("已切到下一首".into())))
}

/// POST /api/admin/playlist/prev — 切到上一首
pub async fn skip_prev(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let cmd = radio_engine::types::AudioCommand {
        cmd_type: radio_engine::types::AudioCommandType::Prev,
        song_id: None,
        file_path: None,
    };

    websocket::publish_command(&state, &cmd).await?;

    Ok(Json(ApiResponse::ok("已切到上一首".into())))
}
