/// 网易云账号设置路由 — 原生 Rust 实现。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, ImportPlaylistRequest, ImportPlaylistResponse, NcmImportTask};
use crate::routes::admin::get_admin;
use crate::services::ncm::{cookie, get_playlist_track_all, NcmClient};
use axum::{extract::State, http::HeaderMap, Json};
use std::sync::Arc;

fn ncm_secrets_path() -> std::path::PathBuf {
    std::env::var("NCM_SECRETS_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("secrets.json"))
}

pub fn read_admin_ncm_cookie() -> Option<String> {
    cookie::read_admin_cookie_from_secrets(&ncm_secrets_path())
        .ok()
        .flatten()
}

/// GET /api/admin/ncm — 获取网易云账号状态
pub async fn get_ncm_settings(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let path = ncm_secrets_path();
    if !path.exists() {
        return Ok(Json(ApiResponse::ok(serde_json::json!({
            "configured": false,
            "method": "none",
            "phone_hint": ""
        }))));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("无法读取 secrets.json")))?;

    let secrets: serde_json::Value =
        serde_json::from_str(&content).unwrap_or(serde_json::Value::Null);

    let configured = secrets
        .get("ncm_cookie")
        .map(|v| cookie::has_cookie(v.as_str().unwrap_or(""), "MUSIC_U"))
        .unwrap_or(false);

    let method = if configured { "cookie" } else { "none" };

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "configured": configured,
        "method": method,
        "phone_hint": ""
    }))))
}

/// POST /api/admin/ncm — 保存网易云账号设置
pub async fn save_ncm_settings(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let path = ncm_secrets_path();
    let mut secrets: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(map) = secrets.as_object_mut() {
        if let Some(cookie) = body.get("cookie").and_then(|v| v.as_str()) {
            if !cookie.is_empty() {
                let cookie = cookie::validate_login_cookie(cookie)
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                map.insert("ncm_cookie".into(), serde_json::Value::String(cookie));
            } else {
                map.remove("ncm_cookie");
            }
        }
        map.remove("ncm_phone");
        map.remove("ncm_password");
    }

    let content = serde_json::to_string_pretty(&secrets)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("序列化失败: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("写入失败: {}", e)))?;

    Ok(Json(ApiResponse::ok("保存成功".into())))
}

/// POST /api/admin/ncm/test — 测试网易云登录
pub async fn test_ncm_login(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let ncm_cookie = match read_admin_ncm_cookie() {
        Some(cookie) => cookie,
        None => {
            return Ok(Json(ApiResponse::ok(serde_json::json!({
                "success": false,
                "output": "未配置网易云账号",
            }))));
        }
    };

    let client = NcmClient::new(None, Some(ncm_cookie));

    match client.test_login().await {
        Ok(true) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": true,
            "output": "登录成功",
        })))),
        Ok(false) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": false,
            "output": "登录失败，Cookie 可能已过期",
        })))),
        Err(e) => Ok(Json(ApiResponse::ok(serde_json::json!({
            "success": false,
            "output": format!("请求失败: {}", e),
        })))),
    }
}

fn extract_playlist_id(link: &str) -> Option<i64> {
    let re = regex::Regex::new(r"(?:id=|/playlist/)(\d+)").ok()?;
    if let Some(caps) = re.captures(link) {
        return caps.get(1)?.as_str().parse().ok();
    }
    link.trim().parse().ok()
}

/// POST /api/admin/ncm/playlist — 解析网易云歌单链接并写入导入任务表
pub async fn import_playlist(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ImportPlaylistRequest>,
) -> Result<Json<ApiResponse<ImportPlaylistResponse>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let playlist_id = extract_playlist_id(&body.link)
        .ok_or_else(|| AppError::BadRequest("无法解析歌单链接".into()))?;

    let ncm_cookie = read_admin_ncm_cookie();
    let client = NcmClient::new(None, ncm_cookie);

    let tracks = get_playlist_track_all(&client, playlist_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("获取歌单失败: {}", e)))?;

    let batch_id = uuid::Uuid::new_v4().to_string();

    for track in &tracks {
        let artist_names = track
            .ar
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        sqlx::query(
            "INSERT INTO ncm_import_tasks (song_id, name, artists, batch_id) VALUES (?, ?, ?, ?)",
        )
        .bind(track.id)
        .bind(&track.name)
        .bind(&artist_names)
        .bind(&batch_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("保存导入任务失败: {}", e)))?;
    }

    Ok(Json(ApiResponse::ok(ImportPlaylistResponse {
        total: tracks.len(),
        batch_id,
        message: format!("成功添加 {} 首歌曲到导入队列", tracks.len()),
    })))
}

/// POST /api/admin/ncm/import — 将 pending 的导入任务加入下载队列
pub async fn start_ncm_import(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let tasks: Vec<NcmImportTask> = sqlx::query_as::<_, NcmImportTask>(
        "SELECT * FROM ncm_import_tasks WHERE status = 'pending'",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("查询导入任务失败: {}", e)))?;

    if tasks.is_empty() {
        return Err(AppError::BadRequest("没有待处理的导入任务".into()));
    }

    // 构建 CSV 格式的歌单文本（与现有下载解析器兼容）
    let mut lines = Vec::new();
    for task in &tasks {
        lines.push(format!("{}, {}", task.artists, task.name));
    }
    let playlist = lines.join("\n");

    // 启动下载任务
    crate::routes::admin::download::spawn_download_job(state.clone(), playlist, None, None)?;

    // 标记为 queued
    for task in &tasks {
        sqlx::query(
            "UPDATE ncm_import_tasks SET status = 'queued', updated_at = datetime('now') WHERE id = ?",
        )
        .bind(task.id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("更新任务状态失败: {}", e)))?;
    }

    Ok(Json(ApiResponse::ok(format!(
        "已启动 {} 首歌曲的导入下载",
        tasks.len()
    ))))
}
