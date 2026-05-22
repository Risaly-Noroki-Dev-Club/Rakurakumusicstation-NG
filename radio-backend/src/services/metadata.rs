use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

/// Re-export engine's parse_artist_title (regex-based, supports multiple dash types).
pub use radio_engine::metadata::parse_artist_title;

/// 查找音频文件旁的封面图片。
pub fn find_cover(audio_path: &Path, media_root: &Path) -> String {
    let cover_names = [
        "cover.jpg",
        "cover.png",
        "cover.jpeg",
        "folder.jpg",
        "folder.png",
        "album.jpg",
        "album.png",
        "front.jpg",
        "front.png",
        "AlbumCover.jpg",
        "AlbumCover.png",
    ];
    let parent = audio_path.parent().unwrap_or_else(|| Path::new("."));

    for name in &cover_names {
        let candidate = parent.join(name);
        if candidate.exists() {
            return candidate
                .strip_prefix(media_root)
                .unwrap_or(&candidate)
                .to_string_lossy()
                .to_string();
        }
    }

    if let Some(stem) = audio_path.file_stem() {
        for ext in &["jpg", "png", "jpeg"] {
            let candidate = parent.join(format!("{}.{}", stem.to_string_lossy(), ext));
            if candidate.exists() {
                return candidate
                    .strip_prefix(media_root)
                    .unwrap_or(&candidate)
                    .to_string_lossy()
                    .to_string();
            }
        }
    }

    String::new()
}

/// Resolve a song cover path, lazily extracting embedded artwork to
/// `media/.covers/{song_id}.jpg` when no sidecar cover is already known.
pub async fn resolve_or_extract_cover(
    db: &SqlitePool,
    song_id: i64,
    file_path: &str,
    cover_path: &str,
    media_root: &Path,
) -> anyhow::Result<Option<String>> {
    if !cover_path.trim().is_empty() {
        let cover_full = media_root.join(cover_path);
        if cover_full.exists() {
            return Ok(Some(cover_path.to_string()));
        }
    }

    let audio_full = media_root.join(file_path);
    if !audio_full.exists() {
        return Ok(None);
    }

    let covers_dir = media_root.join(".covers");
    tokio::fs::create_dir_all(&covers_dir).await?;

    let rel_cover = format!(".covers/{}.jpg", song_id);
    let cover_full = media_root.join(&rel_cover);
    let missing_marker = covers_dir.join(format!("{}.missing", song_id));

    if has_nonempty_file(&cover_full).await {
        update_cover_path(db, song_id, &rel_cover).await?;
        return Ok(Some(rel_cover));
    }
    if missing_marker.exists() {
        return Ok(None);
    }

    let extracted = extract_embedded_cover(&audio_full, &cover_full).await?;
    if extracted {
        let _ = tokio::fs::remove_file(&missing_marker).await;
        update_cover_path(db, song_id, &rel_cover).await?;
        Ok(Some(rel_cover))
    } else {
        let _ = tokio::fs::write(&missing_marker, b"").await;
        Ok(None)
    }
}

async fn has_nonempty_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false)
}

async fn update_cover_path(db: &SqlitePool, song_id: i64, cover_path: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE songs SET cover_path = ? WHERE id = ?")
        .bind(cover_path)
        .bind(song_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn extract_embedded_cover(audio_full: &Path, cover_full: &Path) -> anyhow::Result<bool> {
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-v")
            .arg("error")
            .arg("-i")
            .arg(audio_full)
            .arg("-map")
            .arg("0:v:0")
            .arg("-frames:v")
            .arg("1")
            .arg(cover_full)
            .output(),
    )
    .await;

    match output {
        Ok(Ok(out)) if out.status.success() && has_nonempty_file(cover_full).await => Ok(true),
        Ok(Ok(out)) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stderr.trim().is_empty() {
                tracing::debug!("No embedded cover extracted: {}", stderr.trim());
            }
            let _ = tokio::fs::remove_file(cover_full).await;
            Ok(false)
        }
        Ok(Err(e)) => Err(e.into()),
        Err(_) => {
            let _ = tokio::fs::remove_file(cover_full).await;
            Ok(false)
        }
    }
}

/// 通过 ffprobe 获取音频时长（fork+exec）。
pub fn get_duration(path: &Path) -> Option<i64> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).ok()?;
        let duration_secs: f64 = stdout.trim().parse().ok()?;
        Some((duration_secs * 1000.0) as i64)
    } else {
        None
    }
}

/// 清理文件名，移除路径遍历字符。
pub fn sanitize_filename(filename: &str) -> String {
    filename
        .replace('/', "_")
        .replace('\\', "_")
        .replace("..", "_")
}
