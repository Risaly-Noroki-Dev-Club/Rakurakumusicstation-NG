use std::path::Path;

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
