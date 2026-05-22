use crate::app::state::AppState;
use crate::config::join_base_path;
use std::sync::Arc;

/// GET /api/station — 公开的电台信息
pub async fn station_info(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> axum::Json<serde_json::Value> {
    let ws_host = if state.config.server.host == "0.0.0.0" {
        "localhost"
    } else {
        &state.config.server.host
    };

    let has_admin =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM device_users WHERE role = 'admin'")
            .fetch_one(&state.db)
            .await
            .map(|r| r.0 > 0)
            .unwrap_or(false);

    let station = state.station.read().unwrap_or_else(|e| e.into_inner());

    axum::Json(serde_json::json!({
        "name": station.name,
        "short_name": station.short_name,
        "subtitle": station.subtitle,
        "description": station.description,
        "icon_url": resolved_icon_url(&station, &state.config.server.base_path),
        "manifest_url": join_base_path(&state.config.server.base_path, "/manifest.json"),
        "stream_url": state.config.audio_engine.resolve_stream_url(
            Some(&headers),
            state.config.server.port,
            &state.config.server.base_path,
        ),
        "ws_url": format!("ws://{}:{}/ws", ws_host, state.config.server.port),
        "needs_setup": !has_admin,
    }))
}

fn resolved_icon_url(station: &crate::config::StationConfig, base_path: &str) -> String {
    if !station.icon_path.trim().is_empty() {
        join_base_path(base_path, "/site-icon")
    } else if !station.icon_url.trim().is_empty() {
        station.icon_url.clone()
    } else {
        join_base_path(base_path, "/icon.svg")
    }
}

pub async fn manifest(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    let station = state.station.read().unwrap_or_else(|e| e.into_inner());
    let icon_url = resolved_icon_url(&station, &state.config.server.base_path);
    axum::Json(serde_json::json!({
        "name": station.name,
        "short_name": station.short_name,
        "description": station.description,
        "id": join_base_path(&state.config.server.base_path, "/"),
        "start_url": join_base_path(&state.config.server.base_path, "/"),
        "scope": join_base_path(&state.config.server.base_path, "/"),
        "display": "standalone",
        "background_color": "#FAFAFA",
        "theme_color": "#003D99",
        "icons": [
            {
                "src": icon_url,
                "sizes": "any",
                "type": "image/png",
                "purpose": "any maskable"
            }
        ]
    }))
}
