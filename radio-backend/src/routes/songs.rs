/// 歌曲库路由：搜索、获取歌曲详情、上传、下载。

use crate::auth;
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{ApiResponse, PaginatedResponse, SearchQuery, SongSummary};
use crate::services::metadata::{find_cover, get_duration, sanitize_filename, parse_artist_title};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::get,
    Json, Router,
};
use std::sync::Arc;

pub fn song_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/cover", get(get_song_cover))
        .route("/:id/file", get(stream_song_file))
        .route("/:id/download", get(download_song))
        .route("/:id", get(get_song))
        .route("/", get(search_songs))
        // 上传（带 100MB body limit，需要登录但不限管理员）
        .nest("/upload", Router::new()
            .route("/", axum::routing::post(upload_song))
            .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        )
}

/// GET /api/songs?q=search&limit=20&offset=0
pub async fn search_songs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<SongSummary>>>, AppError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);
    let search = query.q.unwrap_or_default().trim().to_string();

    let (songs, total): (Vec<crate::models::Song>, i64) = if search.is_empty() {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM songs")
            .fetch_one(&state.db)
            .await?;

        let songs = sqlx::query_as::<_, crate::models::Song>(
            "SELECT * FROM songs ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

        (songs, total.0)
    } else {
        let pattern = format!("%{}%", search);
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ?"
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&state.db)
        .await?;

        let songs = sqlx::query_as::<_, crate::models::Song>(
            "SELECT * FROM songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

        (songs, total.0)
    };

    let data: Vec<SongSummary> = songs.into_iter().map(SongSummary::from).collect();

    Ok(Json(ApiResponse::ok(PaginatedResponse {
        total,
        limit,
        offset,
        data,
    })))
}

/// GET /api/songs/{id}
pub async fn get_song(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<crate::models::Song>>, AppError> {
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    Ok(Json(ApiResponse::ok(song)))
}

/// 缺省封面占位 SVG — 简约音乐符图标
const DEFAULT_COVER_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120" fill="none">
  <rect width="120" height="120" rx="8" fill="#e8e8f0"/>
  <path d="M42 85V42l36-8v43" stroke="#999" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>
  <circle cx="38" cy="85" r="7" fill="#bbb"/>
  <circle cx="74" cy="77" r="7" fill="#bbb"/>
</svg>"##;

/// GET /api/songs/{id}/cover — 返回封面图片（JPEG/PNG/SVG 二进制数据）
pub async fn get_song_cover(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    if song.cover_path.is_empty() {
        return Ok(Response::builder()
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(axum::body::Body::from(DEFAULT_COVER_SVG))
            .unwrap());
    }

    let cover_full = std::path::Path::new(&state.config.audio_engine.media_path)
        .join(&song.cover_path);

    let data = std::fs::read(&cover_full)
        .map_err(|_| {
            // 文件丢失时也回退到缺省封面
            DEFAULT_COVER_SVG.to_string()
        });

    match data {
        Ok(bytes) => {
            let mime = match cover_full.extension().and_then(|e| e.to_str()) {
                Some("png") => "image/png",
                _ => "image/jpeg",
            };
            Ok(Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(axum::body::Body::from(bytes))
                .unwrap())
        }
        Err(_) => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(axum::body::Body::from(DEFAULT_COVER_SVG))
            .unwrap()),
    }
}

/// POST /api/songs/upload — 上传音乐文件到媒体目录（需要登录）
pub async fn upload_song(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let _user = auth::require_device_auth(&headers, &state.db).await?;

    let media_path = std::path::PathBuf::from(&state.config.audio_engine.media_path);
    std::fs::create_dir_all(&media_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Create media dir error: {}", e)))?;

    let mut uploaded_filename = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("file").to_string();
        if name != "file" {
            continue;
        }

        let filename = field.file_name()
            .unwrap_or("unknown.mp3")
            .to_string();

        let safe_name = sanitize_filename(&filename);

        let data = field.bytes().await
            .map_err(|e| AppError::BadRequest(format!("读取上传数据失败: {}", e)))?;

        if data.is_empty() {
            return Err(AppError::BadRequest("文件为空".into()));
        }

        let max_size = 100 * 1024 * 1024;
        if data.len() > max_size {
            return Err(AppError::BadRequest("文件大小超过 100MB 限制".into()));
        }

        let dest_path = media_path.join(&safe_name);
        std::fs::write(&dest_path, &data)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("写入文件失败: {}", e)))?;

        uploaded_filename = safe_name.clone();

        let stem = dest_path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(safe_name.clone());
        let (artist, title) = parse_artist_title(&stem);

        let rel_str = safe_name.clone();
        let duration_ms = get_duration(&dest_path).unwrap_or(0);
        let cover_path = find_cover(&dest_path, &media_path);
        let lrc_path = dest_path.with_extension("lrc");
        let lyrics_path = if lrc_path.exists() {
            lrc_path.strip_prefix(&media_path)
                .unwrap_or(&lrc_path)
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };

        sqlx::query(
            "INSERT INTO songs (title, artist, file_path, lyrics_path, cover_path, duration_ms, filesize) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&title)
        .bind(&artist)
        .bind(&rel_str)
        .bind(&lyrics_path)
        .bind(&cover_path)
        .bind(duration_ms)
        .bind(data.len() as i64)
        .execute(&state.db)
        .await?;
    }

    if uploaded_filename.is_empty() {
        return Err(AppError::BadRequest("未找到上传文件字段".into()));
    }

    Ok(Json(ApiResponse::ok(format!("上传成功: {}", uploaded_filename))))
}

/// GET /api/songs/{id}/file — 流式播放歌曲文件（支持 Range 请求，公开）
pub async fn stream_song_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    if song.file_path.is_empty() {
        return Err(AppError::NotFound("Song file not available".into()));
    }

    let file_full = std::path::Path::new(&state.config.audio_engine.media_path)
        .join(&song.file_path);

    if !file_full.exists() {
        return Err(AppError::NotFound("Song file not found on disk".into()));
    }

    let data = std::fs::read(&file_full)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to read file")))?;
    let total_len = data.len() as u64;

    let range_header = headers.get(header::RANGE);
    if let Some(range_val) = range_header {
        let range_str = range_val.to_str().unwrap_or("");
        if let Some(range) = parse_bytes_range(range_str, total_len) {
            let body = axum::body::Body::from(data[range.start as usize..=range.end as usize].to_vec());
            return Ok(Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, "audio/mpeg")
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", range.start, range.end, total_len))
                .header(header::CONTENT_LENGTH, (range.end - range.start + 1).to_string())
                .body(body)
                .unwrap());
        }
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "audio/mpeg")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, total_len.to_string())
        .body(axum::body::Body::from(data))
        .unwrap())
}

struct ByteRange {
    start: u64,
    end: u64,
}

fn parse_bytes_range(range: &str, total: u64) -> Option<ByteRange> {
    let prefix = "bytes=";
    if !range.starts_with(prefix) {
        return None;
    }
    let rest = &range[prefix.len()..];
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let start: u64 = parts[0].parse().ok()?;
    let end: u64 = if parts[1].is_empty() {
        total.saturating_sub(1)
    } else {
        parts[1].parse().ok()?
    };
    if start > end || start >= total {
        return None;
    }
    Some(ByteRange { start, end: end.min(total.saturating_sub(1)) })
}

/// GET /api/songs/{id}/download — 下载歌曲文件（需要登录）
pub async fn download_song(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let _user = auth::require_device_auth(&headers, &state.db).await?;

    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    if song.file_path.is_empty() {
        return Err(AppError::NotFound("Song file not available".into()));
    }

    let file_full = std::path::Path::new(&state.config.audio_engine.media_path)
        .join(&song.file_path);

    if !file_full.exists() {
        return Err(AppError::NotFound("Song file not found on disk".into()));
    }

    let data = std::fs::read(&file_full)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to read file")))?;

    let filename = song.file_path.rsplit('/').next()
        .unwrap_or(&song.file_path)
        .to_string();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "audio/mpeg")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
        .header(header::CONTENT_LENGTH, data.len().to_string())
        .body(axum::body::Body::from(data))
        .unwrap())
}
