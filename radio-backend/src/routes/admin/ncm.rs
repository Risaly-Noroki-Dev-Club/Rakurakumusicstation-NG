/// 网易云账号设置路由 — 原生 Rust 实现。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::{
    ApiResponse, BatchDownloadItem, BatchDownloadResponse, ImportPlaylistRequest,
    ImportPlaylistResponse, NcmImportTask, StartNcmImportRequest,
};
use crate::routes::admin::get_admin;
use crate::services::ncm::{cookie, get_playlist_track_all, get_song_detail, NcmClient};
use axum::{extract::State, http::HeaderMap, Json};
use std::io::Write;
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

fn write_admin_secrets(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    if path.exists() {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
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
    write_admin_secrets(&path, &content)
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

/// 从网易云单曲链接中提取歌曲 id。裸数字保持为歌单 id，
/// 以兼容原有的歌单导入行为。
fn extract_song_id(link: &str) -> Option<i64> {
    let re = regex::Regex::new(r"(?:song\?id=|/song/)(\d+)").ok()?;
    if let Some(caps) = re.captures(link) {
        return caps.get(1)?.as_str().parse().ok();
    }
    None
}

/// POST /api/admin/ncm/playlist — 解析网易云歌单/单曲链接并写入导入任务表
pub async fn import_playlist(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ImportPlaylistRequest>,
) -> Result<Json<ApiResponse<ImportPlaylistResponse>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let ncm_cookie = read_admin_ncm_cookie();
    let client = NcmClient::new(None, ncm_cookie);

    // 统一为 (song_id, name, artists) 列表：歌单链接 → 全曲目；单曲链接 → 1 首。
    let tracks = if let Some(song_id) = extract_song_id(&body.link) {
        let details = get_song_detail(&client, &[song_id])
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("获取歌曲失败: {}", e)))?;
        details
            .into_iter()
            .filter(|d| d.id > 0)
            .map(|d| (d.id, d.name, d.ar))
            .collect::<Vec<_>>()
    } else {
        let playlist_id = extract_playlist_id(&body.link)
            .ok_or_else(|| AppError::BadRequest("无法解析歌单/歌曲链接".into()))?;
        get_playlist_track_all(&client, playlist_id)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("获取歌单失败: {}", e)))?
            .into_iter()
            .map(|t| (t.id, t.name, t.ar))
            .collect::<Vec<_>>()
    };

    if tracks.is_empty() {
        return Err(AppError::BadRequest("未解析到任何歌曲".into()));
    }

    let batch_id = uuid::Uuid::new_v4().to_string();

    for (song_id, name, artists) in &tracks {
        let artist_names = artists
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        sqlx::query(
            "INSERT INTO ncm_import_tasks (song_id, name, artists, batch_id) VALUES (?, ?, ?, ?)",
        )
        .bind(song_id)
        .bind(name)
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
    Json(body): Json<StartNcmImportRequest>,
) -> Result<Json<ApiResponse<BatchDownloadResponse>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let batch_id = body.batch_id.trim();
    if batch_id.is_empty() {
        return Err(AppError::BadRequest("批次号不能为空".into()));
    }

    let tasks: Vec<NcmImportTask> = sqlx::query_as::<_, NcmImportTask>(
        "SELECT * FROM ncm_import_tasks WHERE batch_id = ? AND status = 'pending' ORDER BY id",
    )
    .bind(batch_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("查询导入任务失败: {}", e)))?;

    if tasks.is_empty() {
        return Err(AppError::BadRequest("没有待处理的导入任务".into()));
    }

    let items = tasks
        .iter()
        .map(|task| BatchDownloadItem {
            id: Some(task.song_id.to_string()),
            url: None,
            artist: Some(task.artists.clone()),
            title: Some(task.name.clone()),
            save_as: None,
            override_lyrics: false,
        })
        .collect::<Vec<_>>();

    sqlx::query(
        "UPDATE ncm_import_tasks SET status = 'queued', updated_at = datetime('now') WHERE batch_id = ? AND status = 'pending'",
    )
    .bind(batch_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("更新任务状态失败: {}", e)))?;

    let response = match crate::routes::admin::batch_download::spawn_ncm_batch_job(
        state.clone(),
        items,
        "exhigh".into(),
        "separate".into(),
        Some(batch_id.to_string()),
    ) {
        Ok(response) => response,
        Err(error) => {
            let _ = sqlx::query(
                "UPDATE ncm_import_tasks SET status = 'pending', updated_at = datetime('now') WHERE batch_id = ? AND status = 'queued'",
            )
            .bind(batch_id)
            .execute(&state.db)
            .await;
            return Err(error);
        }
    };

    Ok(Json(ApiResponse::ok(response)))
}

#[cfg(test)]
mod tests {
    use super::{extract_playlist_id, extract_song_id};

    #[test]
    fn parses_explicit_song_links_only() {
        assert_eq!(
            extract_song_id("https://music.163.com/song?id=12345"),
            Some(12345)
        );
        assert_eq!(
            extract_song_id("https://music.163.com/song/67890"),
            Some(67890)
        );
        assert_eq!(extract_song_id("12345"), None);
    }

    #[test]
    fn keeps_bare_numeric_ids_as_playlists() {
        assert_eq!(extract_playlist_id("12345"), Some(12345));
        assert_eq!(
            extract_playlist_id("https://music.163.com/playlist?id=67890"),
            Some(67890)
        );
    }

    #[cfg(unix)]
    #[test]
    fn admin_secrets_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join(format!(
            "radio-ncm-secrets-test-{}.json",
            uuid::Uuid::new_v4()
        ));
        super::write_admin_secrets(&path, r#"{"ncm_cookie":"MUSIC_U=test"}"#).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_file(path).unwrap();
    }
}
