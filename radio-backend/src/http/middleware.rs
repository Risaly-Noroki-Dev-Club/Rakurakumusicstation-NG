//! HTTP middleware that is not tied to a specific route group.

use crate::app::state::AppState;
use axum::{
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// 中间件：确保每个请求都有一个 device_token Cookie。
pub async fn device_cookie_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let is_secure = request_is_secure(request.headers());
    let device_token = crate::auth::extract_device_token(request.headers());
    let new_token = if device_token.is_none() {
        let new_token = crate::auth::generate_device_token();

        if let Err(e) = crate::auth::ensure_device_user(&state.db, &new_token).await {
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
