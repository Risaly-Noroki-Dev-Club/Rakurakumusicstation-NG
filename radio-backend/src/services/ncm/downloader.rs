use super::{api, client::NcmClient};
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Track {
    pub artist: String,
    pub title: String,
    pub raw: String,
}

pub struct DownloadRuntime {
    pub db: sqlx::SqlitePool,
    pub output_dir: String,
    pub concurrency: usize,
}

pub fn parse_playlist(text: &str) -> Vec<Track> {
    let mut tracks = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // CSV format
        if line.contains(',') {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                tracks.push(Track {
                    artist: parts[0].trim().to_string(),
                    title: parts[1].trim().to_string(),
                    raw: line.to_string(),
                });
                continue;
            }
        }
        // "Artist - Title" format
        if let Some(pos) = line.find(" - ") {
            tracks.push(Track {
                artist: line[..pos].trim().to_string(),
                title: line[pos + 3..].trim().to_string(),
                raw: line.to_string(),
            });
        } else {
            tracks.push(Track {
                artist: String::new(),
                title: line.to_string(),
                raw: line.to_string(),
            });
        }
    }
    tracks
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn quality_to_ncm_level(quality: &str) -> &'static str {
    match quality.trim().to_ascii_lowercase().as_str() {
        "standard" | "128k" | "128kbps" => "standard",
        "high" | "higher" | "192k" | "192kbps" => "higher",
        "exhigh" | "320k" | "320kbps" => "exhigh",
        "lossless" | "flac" => "lossless",
        _ => "exhigh",
    }
}

fn ext_from_type(file_type: &str, url: &str) -> &'static str {
    if file_type == "flac" {
        "flac"
    } else if file_type == "mp3" {
        "mp3"
    } else if url.contains(".flac") {
        "flac"
    } else {
        "mp3"
    }
}

async fn download_one(
    client: &NcmClient,
    db: &sqlx::SqlitePool,
    track: &Track,
    quality: &str,
    output_dir: &str,
    log_tx: &Sender<String>,
) -> Result<bool> {
    let keyword = if track.artist.is_empty() {
        track.title.clone()
    } else {
        format!("{} {}", track.artist, track.title)
    };

    log_tx.send(format!("🔍 搜索: {}", keyword)).await.ok();

    // 1. Search
    let results = api::search_song(client, &keyword, 5).await?;
    if results.is_empty() {
        log_tx.send(format!("❌ 未找到: {}", keyword)).await.ok();
        return Ok(false);
    }

    let song = &results[0];
    let detail = api::get_song_detail(client, &[song.id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("未找到歌曲详情"))?;
    let artist_name = detail
        .ar
        .iter()
        .map(|artist| artist.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    log_tx
        .send(format!(
            "✅ 找到: {} - {} (ID: {})",
            artist_name, detail.name, detail.id
        ))
        .await
        .ok();

    // 2. Get download URL
    let level = quality_to_ncm_level(quality);
    let urls = api::get_song_url(client, &[detail.id], level).await?;
    let Some(url_data) = urls.first() else {
        log_tx
            .send(format!("❌ 无法获取下载链接: {}", keyword))
            .await
            .ok();
        return Ok(false);
    };
    let Some(download_url) = url_data.url.as_deref().filter(|url| !url.is_empty()) else {
        log_tx
            .send(format!(
                "❌ 无法获取下载链接: {} (code={})",
                keyword, url_data.code
            ))
            .await
            .ok();
        return Ok(false);
    };
    let ext = ext_from_type(
        url_data.file_type.as_deref().unwrap_or_default(),
        download_url,
    );

    // 3. Download file
    let safe_artist = sanitize_filename(&artist_name);
    let safe_title = sanitize_filename(&detail.name);
    let filename = format!("{} - {}.{}", safe_artist, safe_title, ext);
    let filepath = PathBuf::from(output_dir).join(&filename);

    log_tx
        .send(format!("⬇️ 下载: {} ({} bytes)", filename, url_data.size))
        .await
        .ok();

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let resp = http.get(download_url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("音频文件下载失败: HTTP {}", status);
    }
    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        anyhow::bail!("音频文件下载失败: 返回空文件");
    }

    // MD5 check
    if let Some(expected_md5) = url_data.md5.as_deref().filter(|md5| !md5.is_empty()) {
        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&bytes);
        let file_md5 = format!("{:x}", hasher.finalize());
        if file_md5 != expected_md5 {
            log_tx
                .send(format!(
                    "⚠️ MD5 校验失败: {} (期望 {}, 实际 {})",
                    filename, expected_md5, file_md5
                ))
                .await
                .ok();
        } else {
            log_tx.send("✅ MD5 校验通过".to_string()).await.ok();
        }
    }

    tokio::fs::write(&filepath, &bytes).await?;
    log_tx.send(format!("✅ 已保存: {}", filename)).await.ok();

    // 4. Download lyrics
    let mut saved_lyrics_path = None;
    match api::get_song_lyric(client, detail.id).await {
        Ok(Some(lyric)) if !lyric.is_empty() => {
            let lrc_path = filepath.with_extension("lrc");
            if let Err(e) = tokio::fs::write(&lrc_path, lyric).await {
                log_tx.send(format!("⚠️ 歌词保存失败: {}", e)).await.ok();
            } else {
                saved_lyrics_path = Some(lrc_path.clone());
                log_tx
                    .send(format!("📝 歌词已保存: {}", lrc_path.display()))
                    .await
                    .ok();
            }
        }
        _ => {}
    }

    let library_song_id = super::metadata::sync_downloaded_song(
        db,
        std::path::Path::new(output_dir),
        &filepath,
        saved_lyrics_path.as_deref(),
        &detail,
        bytes.len() as i64,
    )
    .await?;
    log_tx
        .send(format!("📚 已同步到曲库 (歌曲 ID: {})", library_song_id))
        .await
        .ok();

    Ok(true)
}

pub async fn run_download(
    client: NcmClient,
    playlist: String,
    quality: String,
    _format: String,
    runtime: DownloadRuntime,
    log_tx: Sender<String>,
) -> Result<(usize, usize)> {
    let DownloadRuntime {
        db,
        output_dir,
        concurrency,
    } = runtime;
    let tracks = parse_playlist(&playlist);
    let total = tracks.len();
    log_tx
        .send(format!("🎵 共 {} 首歌曲待下载", total))
        .await
        .ok();

    if total == 0 {
        log_tx.send("⚠️ 歌单为空".to_string()).await.ok();
        return Ok((0, 0));
    }

    let mut success = 0usize;
    let mut failed = 0usize;

    if concurrency <= 1 {
        // Serial download
        for (i, track) in tracks.iter().enumerate() {
            log_tx
                .send(format!("--- [{}/{}] {}", i + 1, total, track.raw))
                .await
                .ok();
            match download_one(&client, &db, track, &quality, &output_dir, &log_tx).await {
                Ok(true) => success += 1,
                Ok(false) => failed += 1,
                Err(e) => {
                    log_tx.send(format!("❌ 错误: {}", e)).await.ok();
                    failed += 1;
                }
            }
        }
    } else {
        // Concurrent download with semaphore
        let client = std::sync::Arc::new(client);
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency.min(8)));
        let mut handles = Vec::new();

        for (i, track) in tracks.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = client.clone();
            let db = db.clone();
            let quality = quality.clone();
            let output_dir = output_dir.clone();
            let log_tx = log_tx.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;
                log_tx
                    .send(format!("--- [{}/{}] {}", i + 1, total, track.raw))
                    .await
                    .ok();
                match download_one(&client, &db, &track, &quality, &output_dir, &log_tx).await {
                    Ok(true) => (1usize, 0usize),
                    Ok(false) => (0, 1),
                    Err(e) => {
                        log_tx.send(format!("❌ 错误: {}", e)).await.ok();
                        (0, 1)
                    }
                }
            }));
        }

        for handle in handles {
            let (s, f) = handle.await.unwrap_or((0, 1));
            success += s;
            failed += f;
        }
    }

    log_tx
        .send(format!("🎉 下载完成! 成功: {}, 失败: {}", success, failed))
        .await
        .ok();

    Ok((success, failed))
}
