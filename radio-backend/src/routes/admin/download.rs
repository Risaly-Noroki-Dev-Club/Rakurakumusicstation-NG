/// 批量下载路由。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, DownloadRequest, DownloadStatus};
use crate::routes::admin::get_admin;
use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use std::sync::{Arc, Mutex, OnceLock};

/// 全局下载状态，受 Mutex 保护
fn download_state() -> &'static Mutex<DownloadStatus> {
    static DL: OnceLock<Mutex<DownloadStatus>> = OnceLock::new();
    DL.get_or_init(|| Mutex::new(DownloadStatus {
        running: false,
        log: String::new(),
    }))
}

pub fn ncm_secrets_path() -> std::path::PathBuf {
    std::env::var("NCM_SECRETS_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("secrets.json"))
}

pub fn music_dl_path() -> std::path::PathBuf {
    std::env::var("MUSIC_DL_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("music_dl.py"))
}

/// POST /api/admin/download — 开始批量下载
pub async fn start_download(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DownloadRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    let playlist = body.playlist.trim().to_string();
    if playlist.is_empty() {
        return Err(AppError::BadRequest("歌单内容不能为空".into()));
    }

    {
        let status = download_state().lock().unwrap_or_else(|e| e.into_inner());
        if status.running {
            return Err(AppError::BadRequest("已有下载任务在运行中".into()));
        }
    }

    let quality = body.quality.unwrap_or_else(|| "exhigh".into());
    let format = body.format.unwrap_or_else(|| "mp3".into());

    let tmpdir = std::env::temp_dir();
    let playlist_file = tmpdir.join("radio_download_playlist.txt");
    std::fs::write(&playlist_file, &playlist)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("写入临时文件失败: {}", e)))?;

    {
        let mut status = download_state().lock().unwrap_or_else(|e| e.into_inner());
        status.running = true;
        status.log = format!("开始下载...\n音质: {}\n格式: {}\n", quality, format);
    }

    let media_path = state.config.audio_engine.media_path.clone();
    let quality_clone = quality.clone();
    let format_clone = format.clone();
    let playlist_path = playlist_file.clone();

    let dl_path = music_dl_path();
    let settings_path = ncm_secrets_path();

    tokio::spawn(async move {
        let result = std::process::Command::new("python3")
            .arg(&dl_path)
            .arg(&playlist_path)
            .arg("--output")
            .arg(&media_path)
            .arg("--quality")
            .arg(&quality_clone)
            .arg("--format")
            .arg(&format_clone)
            .arg("--non-interactive")
            .arg("--settings")
            .arg(&settings_path)
            .output();

        let mut status = download_state().lock().unwrap_or_else(|e| e.into_inner());
        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                status.log = format!("{}\n{}", stdout, stderr);
                status.running = false;
                std::fs::remove_file(&playlist_path).ok();
            }
            Err(e) => {
                status.log = format!("下载失败: {}", e);
                status.running = false;
            }
        }
    });

    Ok(Json(ApiResponse::ok("下载任务已启动".into())))
}

/// GET /api/admin/download/status — 获取下载状态
pub async fn download_status(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DownloadStatus>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let status = download_state().lock().unwrap_or_else(|e| e.into_inner());
    Ok(Json(ApiResponse::ok(status.clone())))
}
