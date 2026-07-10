/// 退出登录路由。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::ApiResponse;
use crate::routes::admin::get_admin;
use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

/// POST /api/admin/logout — 清除当前设备的身份 Cookie。
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let _admin = get_admin(&state, &headers).await?;
    let secure_attr = if request_is_secure(&headers) {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "device_token=; Path={}; HttpOnly; SameSite=Lax; Max-Age=0{}",
        state.config.server.base_path, secure_attr
    );

    let mut response = Json(ApiResponse::<String>::ok("已退出".into())).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie
            .parse()
            .map_err(|e| AppError::Internal(anyhow::Error::new(e)))?,
    );
    Ok(response)
}

fn request_is_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}
