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
    match quality {
        "standard" => "standard",
        "high" => "higher",
        "exhigh" => "exhigh",
        "lossless" => "lossless",
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
    let artist_name = song
        .artists
        .first()
        .map(|a| a.name.as_str())
        .unwrap_or("")
        .to_string();
    log_tx
        .send(format!(
            "✅ 找到: {} - {} (ID: {})",
            artist_name, song.name, song.id
        ))
        .await
        .ok();

    // 2. Get download URL
    let level = quality_to_ncm_level(quality);
    let urls = api::get_song_url(client, &[song.id], level).await?;
    if urls.is_empty() || urls[0].url.is_empty() {
        log_tx
            .send(format!("❌ 无法获取下载链接: {}", keyword))
            .await
            .ok();
        return Ok(false);
    }

    let url_data = &urls[0];
    let ext = ext_from_type(&url_data.file_type, &url_data.url);

    // 3. Download file
    let safe_artist = sanitize_filename(&artist_name);
    let safe_title = sanitize_filename(&song.name);
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
    let resp = http.get(&url_data.url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("音频文件下载失败: HTTP {}", status);
    }
    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        anyhow::bail!("音频文件下载失败: 返回空文件");
    }

    // MD5 check
    if !url_data.md5.is_empty() {
        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&bytes);
        let file_md5 = format!("{:x}", hasher.finalize());
        if file_md5 != url_data.md5 {
            log_tx
                .send(format!(
                    "⚠️ MD5 校验失败: {} (期望 {}, 实际 {})",
                    filename, url_data.md5, file_md5
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
    match api::get_song_lyric(client, song.id).await {
        Ok(Some(lyric)) if !lyric.is_empty() => {
            let lrc_path = filepath.with_extension("lrc");
            if let Err(e) = tokio::fs::write(&lrc_path, lyric).await {
                log_tx.send(format!("⚠️ 歌词保存失败: {}", e)).await.ok();
            } else {
                log_tx
                    .send(format!("📝 歌词已保存: {}", lrc_path.display()))
                    .await
                    .ok();
            }
        }
        _ => {}
    }

    Ok(true)
}

pub async fn run_download(
    client: NcmClient,
    playlist: String,
    quality: String,
    _format: String,
    output_dir: String,
    concurrency: usize,
    log_tx: Sender<String>,
) -> Result<(usize, usize)> {
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
            match download_one(&client, track, &quality, &output_dir, &log_tx).await {
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
            let quality = quality.clone();
            let output_dir = output_dir.clone();
            let log_tx = log_tx.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit;
                log_tx
                    .send(format!("--- [{}/{}] {}", i + 1, total, track.raw))
                    .await
                    .ok();
                match download_one(&client, &track, &quality, &output_dir, &log_tx).await {
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
