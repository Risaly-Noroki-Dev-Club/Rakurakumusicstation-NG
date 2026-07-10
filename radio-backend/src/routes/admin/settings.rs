/// 系统设置路由。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, SaveSettingsRequest, SettingsResponse};
use crate::routes::admin::get_admin;
use axum::{
    extract::{Multipart, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use std::{
    io::Write,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

const MAX_ICON_SIZE: usize = 2 * 1024 * 1024;

fn config_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn load_config(path: &str) -> Result<toml::Value, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Read config error: {}", e)))?;
    let value: toml::Value = toml::from_str(&content)
        .map_err(|e| AppError::BadRequest(format!("config.toml 格式无效，未保存更改: {}", e)))?;

    if !value.is_table() {
        return Err(AppError::BadRequest(
            "config.toml 根节点必须是表，未保存更改".into(),
        ));
    }
    Ok(value)
}

fn write_config_atomically(path: &str, value: &toml::Value) -> Result<(), AppError> {
    let content = toml::to_string_pretty(value)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TOML serialize error: {}", e)))?;
    let path = Path::new(path);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::BadRequest("配置文件路径无效".into()))?;
    let temporary_path = parent.join(format!(".{}.{}.tmp", file_name, uuid::Uuid::new_v4()));

    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path)
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(AppError::Internal(anyhow::anyhow!(
            "Write config error: {}",
            error
        )));
    }
    Ok(())
}

fn resolved_icon_url(station: &crate::config::StationConfig, base_path: &str) -> String {
    if !station.icon_path.trim().is_empty() {
        crate::config::join_base_path(base_path, "/site-icon")
    } else if !station.icon_url.trim().is_empty() {
        station.icon_url.clone()
    } else {
        crate::config::join_base_path(base_path, "/icon.svg")
    }
}

fn icon_content_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "image/png",
    }
}

fn allowed_icon_extension(filename: &str, content_type: Option<&str>) -> Option<String> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "svg" | "webp" | "jpg" => Some(ext),
        "jpeg" => Some("jpg".to_string()),
        _ => match content_type.unwrap_or("") {
            "image/png" => Some("png".to_string()),
            "image/svg+xml" => Some("svg".to_string()),
            "image/webp" => Some("webp".to_string()),
            "image/jpeg" => Some("jpg".to_string()),
            _ => None,
        },
    }
}

/// GET /api/admin/settings — 获取系统设置
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<SettingsResponse>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let station = state.station.read().unwrap_or_else(|e| e.into_inner());
    Ok(Json(ApiResponse::ok(SettingsResponse {
        station_name: station.name.clone(),
        short_name: station.short_name.clone(),
        subtitle: station.subtitle.clone(),
        description: station.description.clone(),
        icon_url: station.icon_url.clone(),
        icon_path: station.icon_path.clone(),
        resolved_icon_url: resolved_icon_url(&station, &state.config.server.base_path),
    })))
}

/// POST /api/admin/settings — 保存系统设置（写入 config.toml）
pub async fn save_settings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SaveSettingsRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    let config_path = std::env::var("RADIO_CONFIG").unwrap_or_else(|_| "config.toml".to_string());

    {
        let _config_guard = config_write_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut toml_value = load_config(&config_path)?;
        let toml::Value::Table(root) = &mut toml_value else {
            unreachable!("load_config validates the root table");
        };
        let station = root
            .entry("station")
            .or_insert(toml::Value::Table(toml::value::Table::new()));
        if let toml::Value::Table(ref mut st) = station {
            if let Some(ref v) = body.station_name {
                st.insert("name".into(), toml::Value::String(v.clone()));
            }
            if let Some(ref v) = body.short_name {
                st.insert("short_name".into(), toml::Value::String(v.clone()));
            }
            if let Some(ref v) = body.subtitle {
                st.insert("subtitle".into(), toml::Value::String(v.clone()));
            }
            if let Some(ref v) = body.description {
                st.insert("description".into(), toml::Value::String(v.clone()));
            }
            if let Some(ref v) = body.icon_url {
                st.insert("icon_url".into(), toml::Value::String(v.clone()));
            }
        }
        write_config_atomically(&config_path, &toml_value)?;
    }

    {
        let mut station = state.station.write().unwrap_or_else(|e| e.into_inner());
        if let Some(ref v) = body.station_name {
            station.name = v.clone();
        }
        if let Some(ref v) = body.short_name {
            station.short_name = v.clone();
        }
        if let Some(ref v) = body.subtitle {
            station.subtitle = v.clone();
        }
        if let Some(ref v) = body.description {
            station.description = v.clone();
        }
        if let Some(ref v) = body.icon_url {
            station.icon_url = v.clone();
        }
    }

    sqlx::query(
        "INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'update_settings', ?)",
    )
    .bind(admin.id)
    .bind("Updated system settings")
    .execute(&state.db)
    .await?;

    Ok(Json(ApiResponse::ok("设置已保存，立即生效".into())))
}

/// POST /api/admin/settings/icon — upload site icon used by favicon/manifest.
pub async fn upload_icon(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let admin = get_admin(&state, &headers).await?;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") != "file" {
            continue;
        }

        let filename = field.file_name().unwrap_or("site-icon.png").to_string();
        let content_type = field.content_type().map(|s| s.to_string());
        let ext = allowed_icon_extension(&filename, content_type.as_deref())
            .ok_or_else(|| AppError::BadRequest("仅支持 PNG、SVG、WebP、JPEG 图标".into()))?;
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("读取上传图标失败: {}", e)))?;
        if data.is_empty() {
            return Err(AppError::BadRequest("图标文件为空".into()));
        }
        if data.len() > MAX_ICON_SIZE {
            return Err(AppError::BadRequest("图标文件超过 2MB 限制".into()));
        }

        let dir = std::path::PathBuf::from("data");
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Create icon dir error: {}", e)))?;
        let rel_path = format!("data/site-icon.{}", ext);
        std::fs::write(&rel_path, &data)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Write icon error: {}", e)))?;

        let config_path =
            std::env::var("RADIO_CONFIG").unwrap_or_else(|_| "config.toml".to_string());
        {
            let _config_guard = config_write_lock()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut toml_value = load_config(&config_path)?;
            let toml::Value::Table(root) = &mut toml_value else {
                unreachable!("load_config validates the root table");
            };
            let station = root
                .entry("station")
                .or_insert(toml::Value::Table(toml::value::Table::new()));
            if let toml::Value::Table(ref mut st) = station {
                st.insert("icon_path".into(), toml::Value::String(rel_path.clone()));
            }
            write_config_atomically(&config_path, &toml_value)?;
        }

        {
            let mut station = state.station.write().unwrap_or_else(|e| e.into_inner());
            station.icon_path = rel_path.clone();
        }

        sqlx::query(
            "INSERT INTO admin_log (admin_id, action, details) VALUES (?, 'upload_icon', ?)",
        )
        .bind(admin.id)
        .bind(format!("Uploaded site icon {}", rel_path))
        .execute(&state.db)
        .await?;

        return Ok(Json(ApiResponse::ok("图标已上传".into())));
    }

    Err(AppError::BadRequest("未找到上传文件字段".into()))
}

pub async fn site_icon(State(state): State<Arc<AppState>>) -> Result<Response, AppError> {
    let station = state.station.read().unwrap_or_else(|e| e.into_inner());
    if station.icon_path.trim().is_empty() {
        return Err(AppError::NotFound("未配置上传图标".into()));
    }
    let path = std::path::PathBuf::from(&station.icon_path);
    drop(station);
    let data = std::fs::read(&path).map_err(|_| AppError::NotFound("图标文件不存在".into()))?;
    let content_type = icon_content_type(&path);
    let is_svg = content_type == "image/svg+xml";

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache")
        .header("x-content-type-options", "nosniff");

    if is_svg {
        builder = builder.header("content-security-policy", "sandbox");
    }

    Ok(builder.body(axum::body::Body::from(data)).unwrap())
}
