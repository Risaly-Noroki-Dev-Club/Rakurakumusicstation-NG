use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Re-export engine's parse_artist_title (regex-based, supports multiple dash types).
pub use radio_engine::metadata::parse_artist_title;

#[derive(Debug, Default)]
pub struct LocalAudioMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: i64,
    pub filesize: i64,
    pub title_from_tag: bool,
    pub artist_from_tag: bool,
    pub album_from_tag: bool,
    pub embedded_lyrics: String,
    pub has_embedded_cover: bool,
}

#[derive(Deserialize)]
struct ProbeOutput {
    #[serde(default)]
    format: ProbeFormat,
    #[serde(default)]
    streams: Vec<ProbeStream>,
}

#[derive(Default, Deserialize)]
struct ProbeFormat {
    #[serde(default)]
    duration: String,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Default, Deserialize)]
struct ProbeStream {
    #[serde(default)]
    codec_type: String,
    #[serde(default)]
    disposition: ProbeDisposition,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Default, Deserialize)]
struct ProbeDisposition {
    #[serde(default)]
    attached_pic: i32,
}

/// 读取本地音频标签和时长；缺失的标题、艺术家回退到文件名。
pub fn read_local_metadata(path: &Path) -> LocalAudioMetadata {
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let (filename_artist, filename_title) = parse_artist_title(&stem);
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:format_tags:stream=codec_type:stream_disposition=attached_pic:stream_tags",
            "-of",
            "json",
        ])
        .arg(path)
        .output();
    let Ok(output) = output else {
        return LocalAudioMetadata {
            title: filename_title,
            artist: filename_artist,
            ..Default::default()
        };
    };
    if !output.status.success() {
        return LocalAudioMetadata {
            title: filename_title,
            artist: filename_artist,
            ..Default::default()
        };
    }

    let Ok(probe) = serde_json::from_slice::<ProbeOutput>(&output.stdout) else {
        return LocalAudioMetadata {
            title: filename_title,
            artist: filename_artist,
            ..Default::default()
        };
    };
    let tag = |names: &[&str]| find_tag(&probe, names);
    let tagged_title = tag(&["title"]);
    let tagged_artist = tag(&["artist", "albumartist", "album_artist"]);
    let tagged_album = tag(&["album"]);
    let embedded_lyrics = tag(&[
        "syncedlyrics",
        "lyrics",
        "unsyncedlyrics",
        "lyrics-eng",
        "lyrics-chi",
    ]);

    LocalAudioMetadata {
        title: if tagged_title.is_empty() {
            filename_title
        } else {
            tagged_title.clone()
        },
        artist: if tagged_artist.is_empty() {
            filename_artist
        } else {
            tagged_artist.clone()
        },
        album: tagged_album.clone(),
        duration_ms: probe
            .format
            .duration
            .parse::<f64>()
            .map(|seconds| (seconds * 1000.0) as i64)
            .unwrap_or(0),
        filesize: fs::metadata(path)
            .map(|value| value.len() as i64)
            .unwrap_or(0),
        title_from_tag: !tagged_title.is_empty(),
        artist_from_tag: !tagged_artist.is_empty(),
        album_from_tag: !tagged_album.is_empty(),
        embedded_lyrics,
        has_embedded_cover: probe
            .streams
            .iter()
            .any(|stream| stream.codec_type == "video" && stream.disposition.attached_pic != 0),
    }
}

fn find_tag(probe: &ProbeOutput, names: &[&str]) -> String {
    probe
        .format
        .tags
        .iter()
        .chain(probe.streams.iter().flat_map(|stream| stream.tags.iter()))
        .find(|(key, value)| {
            !value.trim().is_empty() && names.iter().any(|name| key.eq_ignore_ascii_case(name))
        })
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default()
}

/// 查找音频文件旁的封面图片。
pub fn find_cover(audio_path: &Path, media_root: &Path) -> String {
    let parent = audio_path.parent().unwrap_or_else(|| Path::new("."));
    let audio_stem = audio_path
        .file_stem()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let preferred = [
        audio_stem.as_str(),
        "cover",
        "folder",
        "album",
        "front",
        "albumcover",
    ];
    if let Ok(entries) = fs::read_dir(parent) {
        let mut candidates = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| {
                matches!(
                    path.extension()
                        .and_then(|value| value.to_str())
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .as_str(),
                    "jpg" | "jpeg" | "png" | "webp"
                )
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|path| {
            let stem = path
                .file_stem()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            preferred
                .iter()
                .position(|value| *value == stem)
                .unwrap_or(usize::MAX)
        });
        if let Some(candidate) = candidates.into_iter().find(|path| {
            let stem = path
                .file_stem()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            preferred.contains(&stem.as_str())
        }) {
            return candidate
                .strip_prefix(media_root)
                .unwrap_or(&candidate)
                .to_string_lossy()
                .to_string();
        }
    }

    String::new()
}

/// Resolve a song cover path, lazily extracting embedded artwork to
/// `media/.covers/{song_id}.jpg` when no sidecar cover is already known.
///
/// Called both on demand (GET /api/songs/:id/cover) and eagerly right after
/// a song is ingested (upload / rescan), so cover requests hit the cache.
pub async fn ensure_cover_cached(
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
    let fingerprint = file_fingerprint(&audio_full).await.unwrap_or_default();
    if tokio::fs::read_to_string(&missing_marker)
        .await
        .is_ok_and(|stored| stored == fingerprint)
    {
        return Ok(None);
    }

    // 先用 ffprobe 快速探测是否真的存在封面流；没有就直接落 .missing
    // 标记并返回，避免对每首无封面歌曲都跑一次最长 30s 的 ffmpeg 提取。
    // 探测/提取是 CPU 密集的子进程操作：用全局信号量限制并发，
    // 防止 rescan 大批量入库时同时 fork 数十个 ffmpeg/ffprobe。
    let _permit = COVER_PROBE_SEM
        .acquire()
        .await
        .map_err(|_| anyhow::anyhow!("cover semaphore closed"))?;
    if !has_cover_stream(&audio_full).await? {
        let _ = tokio::fs::write(&missing_marker, fingerprint.as_bytes()).await;
        return Ok(None);
    }

    let extracted = extract_embedded_cover(&audio_full, &cover_full).await?;
    if extracted {
        let _ = tokio::fs::remove_file(&missing_marker).await;
        update_cover_path(db, song_id, &rel_cover).await?;
        Ok(Some(rel_cover))
    } else {
        let _ = tokio::fs::write(&missing_marker, fingerprint.as_bytes()).await;
        Ok(None)
    }
}

async fn file_fingerprint(path: &Path) -> Option<String> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("{}:{}", metadata.len(), modified))
}

/// 封面探测/提取的并发上限（ffprobe/ffmpeg 子进程）。
static COVER_PROBE_SEM: std::sync::LazyLock<tokio::sync::Semaphore> =
    std::sync::LazyLock::new(|| tokio::sync::Semaphore::new(4));

/// 快速探测音频文件是否内嵌封面流（attached pic 在 ffprobe 中是 video 流）。
async fn has_cover_stream(audio_full: &Path) -> anyhow::Result<bool> {
    let output = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-select_streams")
            .arg("v")
            .arg("-show_entries")
            .arg("stream=codec_type")
            .arg("-of")
            .arg("csv=p=0")
            .arg(audio_full)
            .output(),
    )
    .await;

    match output {
        Ok(Ok(out)) if out.status.success() => {
            Ok(String::from_utf8_lossy(&out.stdout).contains("video"))
        }
        // ffprobe 不可用/失败：保守起见当作没有封面（快速路径），
        // 封面提取仍然可以事后通过显式上传补充。
        _ => Ok(false),
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
    let temporary = cover_full.with_extension("part.jpg");
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
            .arg(&temporary)
            .output(),
    )
    .await;

    match output {
        Ok(Ok(out)) if out.status.success() && has_nonempty_file(&temporary).await => {
            tokio::fs::rename(&temporary, cover_full).await?;
            Ok(true)
        }
        Ok(Ok(out)) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stderr.trim().is_empty() {
                tracing::debug!("No embedded cover extracted: {}", stderr.trim());
            }
            let _ = tokio::fs::remove_file(&temporary).await;
            Ok(false)
        }
        Ok(Err(e)) => Err(e.into()),
        Err(_) => {
            let _ = tokio::fs::remove_file(&temporary).await;
            Ok(false)
        }
    }
}

/// Find a sidecar LRC case-insensitively, preferring an exact stem match.
pub fn find_lyrics(audio_path: &Path, media_root: &Path) -> String {
    let parent = audio_path.parent().unwrap_or_else(|| Path::new("."));
    let wanted = audio_path
        .file_stem()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let found = fs::read_dir(parent).ok().and_then(|entries| {
        entries.flatten().map(|entry| entry.path()).find(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("lrc"))
                && path
                    .file_stem()
                    .map(|value| value.to_string_lossy().to_ascii_lowercase() == wanted)
                    .unwrap_or(false)
        })
    });
    found
        .map(|path| {
            path.strip_prefix(media_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_default()
}

/// Persist embedded or downloaded lyrics in the managed lyrics cache.
pub async fn cache_lyrics(
    media_root: &Path,
    song_id: i64,
    content: &str,
) -> anyhow::Result<String> {
    let directory = media_root.join(".lyrics");
    tokio::fs::create_dir_all(&directory).await?;
    let relative = format!(".lyrics/{}.lrc", song_id);
    let destination = media_root.join(&relative);
    let temporary = directory.join(format!("{}.lrc.part", song_id));
    tokio::fs::write(&temporary, content.as_bytes()).await?;
    tokio::fs::rename(&temporary, destination).await?;
    Ok(relative)
}

pub fn cover_mime(path: &Path, bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else {
        match path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => "image/png",
            "webp" => "image/webp",
            _ => "image/jpeg",
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
