/// 系统设置路由。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, SaveSettingsRequest, SettingsResponse};
use crate::routes::admin::get_admin;
use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

/// GET /api/admin/settings — 获取系统设置
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<SettingsResponse>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let station = state.station.read().unwrap_or_else(|e| e.into_inner());
    Ok(Json(ApiResponse::ok(SettingsResponse {
        station_name: station.name.clone(),
        subtitle: station.subtitle.clone(),
        primary_color: station.primary_color.clone(),
        secondary_color: station.secondary_color.clone(),
        bg_color: station.bg_color.clone(),
    })))
}

/// POST /api/admin/settings — 保存系统设置（写入 config.toml）
pub async fn save_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SaveSettingsRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    let config_path = std::env::var("RADIO_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());

    let mut toml_value: toml::Value = {
        let content = std::fs::read_to_string(&config_path)
            .unwrap_or_default();
        toml::from_str(&content).unwrap_or(toml::Value::Table(toml::value::Table::new()))
    };

    if let toml::Value::Table(ref mut root) = toml_value {
        let station = root.entry("station")
            .or_insert(toml::Value::Table(toml::value::Table::new()));
        if let toml::Value::Table(ref mut st) = station {
            if let Some(ref v) = body.station_name { st.insert("name".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.subtitle { st.insert("subtitle".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.primary_color { st.insert("primary_color".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.secondary_color { st.insert("secondary_color".into(), toml::Value::String(v.clone())); }
            if let Some(ref v) = body.bg_color { st.insert("bg_color".into(), toml::Value::String(v.clone())); }
        }
    }

    std::fs::write(&config_path, toml::to_string_pretty(&toml_value)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TOML serialize error: {}", e)))?)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Write config error: {}", e)))?;

    {
        let mut station = state.station.write().unwrap_or_else(|e| e.into_inner());
        if let Some(ref v) = body.station_name { station.name = v.clone(); }
        if let Some(ref v) = body.subtitle { station.subtitle = v.clone(); }
        if let Some(ref v) = body.primary_color { station.primary_color = v.clone(); }
        if let Some(ref v) = body.secondary_color { station.secondary_color = v.clone(); }
        if let Some(ref v) = body.bg_color { station.bg_color = v.clone(); }
    }

    sqlx::query("INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'update_settings', ?)")
        .bind(admin.id)
        .bind("Updated system settings")
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::ok("设置已保存，立即生效".into())))
}
