use std::path::Path;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::Serialize;
use sqlx::SqlitePool;

use super::api;
use super::types::SongDetailData;
use crate::models::Song;

const MATCH_THRESHOLD: i32 = 75;
const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Default, Serialize)]
pub struct EnrichReport {
    pub matched: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// 匿名查询网易云，只为缺少专辑或封面的本地歌曲补全元数据。
pub async fn enrich_library(db: &SqlitePool, media_root: &Path) -> Result<EnrichReport> {
    let songs = sqlx::query_as::<_, Song>(
        "SELECT * FROM songs WHERE file_path != '' AND ncm_song_id IS NULL AND (album = '' OR cover_path = '') ORDER BY id",
    )
    .fetch_all(db)
    .await?;
    let mut report = EnrichReport::default();

    for song in songs {
        match enrich_song(db, media_root, &song).await {
            Ok(true) => report.matched += 1,
            Ok(false) => report.skipped += 1,
            Err(error) => {
                report.failed += 1;
                tracing::warn!(song_id = song.id, ?error, "网易云元数据补全失败");
            }
        }
    }

    Ok(report)
}

async fn enrich_song(db: &SqlitePool, media_root: &Path, song: &Song) -> Result<bool> {
    let keyword = if song.artist.trim().is_empty() {
        song.title.clone()
    } else {
        format!("{} {}", song.title, song.artist)
    };
    let candidates = api::search_song_anonymous(&keyword, 10).await?;
    let Some(candidate) = candidates
        .into_iter()
        .max_by_key(|candidate| match_score(song, candidate))
    else {
        return Ok(false);
    };

    let score = match_score(song, &candidate);
    if score < MATCH_THRESHOLD || !artist_matches(song, &candidate) {
        tracing::info!(
            song_id = song.id,
            ncm_song_id = candidate.id,
            score,
            "网易云候选匹配度不足"
        );
        return Ok(false);
    }

    let detail = api::get_song_detail_anonymous(candidate.id)
        .await?
        .context("网易云歌曲详情为空")?;

    let cover_path = if song.cover_path.trim().is_empty() && !detail.al.pic_url.trim().is_empty() {
        download_cover(media_root, song.id, &detail.al.pic_url).await?
    } else {
        song.cover_path.clone()
    };
    let album = if song.album.trim().is_empty() {
        detail.al.name
    } else {
        song.album.clone()
    };

    sqlx::query(
        "UPDATE songs SET album = ?, cover_path = ?, ncm_song_id = ?, metadata_source = 'ncm', metadata_matched_at = datetime('now') WHERE id = ?",
    )
    .bind(album)
    .bind(cover_path)
    .bind(detail.id)
    .bind(song.id)
    .execute(db)
    .await?;

    Ok(true)
}

/// Persist a downloaded NCM track in the local library while retaining the
/// exact upstream song ID used for the download. Cover failures are reported
/// to the caller but do not undo the already downloaded audio or its metadata.
pub async fn sync_downloaded_song(
    db: &SqlitePool,
    media_root: &Path,
    audio_path: &Path,
    lyrics_path: Option<&Path>,
    detail: &SongDetailData,
    filesize: i64,
) -> Result<i64> {
    let file_path = radio_engine::util::relativize_media_path(audio_path, media_root);
    let incoming_lyrics_path = lyrics_path
        .filter(|path| path.exists())
        .map(|path| radio_engine::util::relativize_media_path(path, media_root))
        .unwrap_or_default();
    let artist = detail
        .ar
        .iter()
        .map(|artist| artist.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    let mut conn = db.acquire().await?;
    let existing = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, lyrics_path, cover_path FROM songs WHERE file_path = ? ORDER BY id LIMIT 1",
    )
    .bind(&file_path)
    .fetch_optional(&mut *conn)
    .await?;

    let (song_id, lyrics_path, existing_cover_path) = if let Some((id, old_lyrics, cover)) =
        existing
    {
        let lyrics = if incoming_lyrics_path.is_empty() {
            old_lyrics
        } else {
            incoming_lyrics_path
        };
        sqlx::query(
            "UPDATE songs SET title = ?, artist = ?, album = ?, duration_ms = ?, lyrics_path = ?, filesize = ?, ncm_song_id = ?, metadata_source = 'ncm_download', metadata_matched_at = datetime('now') WHERE id = ?",
        )
        .bind(&detail.name)
        .bind(&artist)
        .bind(&detail.al.name)
        .bind(detail.dt)
        .bind(&lyrics)
        .bind(filesize)
        .bind(detail.id)
        .bind(id)
        .execute(&mut *conn)
        .await?;
        (id, lyrics, cover)
    } else {
        sqlx::query(
            "INSERT INTO songs (title, artist, album, duration_ms, file_path, lyrics_path, cover_path, filesize, ncm_song_id, metadata_source, metadata_matched_at) VALUES (?, ?, ?, ?, ?, ?, '', ?, ?, 'ncm_download', datetime('now'))",
        )
        .bind(&detail.name)
        .bind(&artist)
        .bind(&detail.al.name)
        .bind(detail.dt)
        .bind(&file_path)
        .bind(&incoming_lyrics_path)
        .bind(filesize)
        .bind(detail.id)
        .execute(&mut *conn)
        .await?;
        let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(&mut *conn)
            .await?;
        (id, incoming_lyrics_path, String::new())
    };
    drop(conn);

    if existing_cover_path.is_empty() && !detail.al.pic_url.trim().is_empty() {
        match download_cover(media_root, song_id, &detail.al.pic_url).await {
            Ok(cover_path) => {
                sqlx::query("UPDATE songs SET cover_path = ? WHERE id = ?")
                    .bind(cover_path)
                    .bind(song_id)
                    .execute(db)
                    .await?;
            }
            Err(error) => {
                tracing::warn!(song_id, ?error, "网易云封面下载失败，歌曲元数据已保留");
            }
        }
    }

    tracing::info!(
        song_id,
        ncm_song_id = detail.id,
        file_path,
        lyrics_path,
        "网易云下载元数据已同步到曲库"
    );
    Ok(song_id)
}

fn match_score(song: &Song, candidate: &super::types::SearchSongItem) -> i32 {
    let title = normalize(&song.title);
    let candidate_title = normalize(&candidate.name);
    let mut score = if !title.is_empty() && title == candidate_title {
        60
    } else if !title.is_empty()
        && !candidate_title.is_empty()
        && (title.contains(&candidate_title) || candidate_title.contains(&title))
    {
        35
    } else {
        0
    };

    if !normalize(&song.artist).is_empty() && artist_matches(song, candidate) {
        score += 25;
    }

    if song.duration_ms > 0 && candidate.duration > 0 {
        let difference = (song.duration_ms - candidate.duration).abs();
        score += match difference {
            0..=2_000 => 20,
            2_001..=5_000 => 15,
            5_001..=10_000 => 8,
            _ => 0,
        };
    }

    score
}

fn artist_matches(song: &Song, candidate: &super::types::SearchSongItem) -> bool {
    let artist = normalize(&song.artist);
    artist.is_empty()
        || candidate.artists.iter().any(|item| {
            let candidate_artist = normalize(&item.name);
            !candidate_artist.is_empty()
                && (artist == candidate_artist
                    || artist.contains(&candidate_artist)
                    || candidate_artist.contains(&artist))
        })
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

pub(crate) async fn download_cover(media_root: &Path, song_id: i64, url: &str) -> Result<String> {
    let parsed_url = reqwest::Url::parse(url).context("网易云封面 URL 无效")?;
    let host = parsed_url.host_str().unwrap_or_default();
    if parsed_url.scheme() != "https"
        || !(host == "music.126.net"
            || host.ends_with(".music.126.net")
            || host == "music.163.com"
            || host.ends_with(".music.163.com"))
    {
        anyhow::bail!("网易云返回了不受信任的封面地址");
    }
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .get(parsed_url)
        .send()
        .await?
        .error_for_status()?;
    let is_image = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("image/"));
    if !is_image {
        anyhow::bail!("网易云封面响应不是图片");
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_COVER_BYTES as u64)
    {
        anyhow::bail!("网易云封面超过 10MB 上限");
    }

    let covers_dir = media_root.join(".covers");
    tokio::fs::create_dir_all(&covers_dir).await?;
    let relative = format!(".covers/{}.jpg", song_id);
    let destination = media_root.join(&relative);
    let temporary = covers_dir.join(format!("{}.jpg.part", song_id));
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len() + chunk.len() > MAX_COVER_BYTES {
            anyhow::bail!("网易云封面超过 10MB 上限");
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        anyhow::bail!("网易云封面为空");
    }

    tokio::fs::write(&temporary, bytes).await?;
    tokio::fs::rename(&temporary, &destination).await?;
    let _ = tokio::fs::remove_file(covers_dir.join(format!("{}.missing", song_id))).await;
    Ok(relative)
}

#[cfg(test)]
mod tests {
    use super::{artist_matches, match_score, normalize, sync_downloaded_song};
    use crate::models::Song;
    use crate::services::ncm::types::{SearchAlbum, SearchArtist, SearchSongItem};
    use chrono::Utc;

    fn local_song() -> Song {
        Song {
            id: 1,
            title: "合唱团，阴郁的夜晚".into(),
            artist: "v是兔子wishtoday".into(),
            album: String::new(),
            genre: String::new(),
            year: 0,
            duration_ms: 234_000,
            file_path: "test.flac".into(),
            lyrics_path: String::new(),
            cover_path: String::new(),
            filesize: 0,
            created_at: Utc::now().naive_utc(),
            ncm_song_id: None,
            metadata_source: String::new(),
            metadata_matched_at: None,
        }
    }

    #[test]
    fn normalizes_punctuation_and_case() {
        assert_eq!(normalize("Hello， World!"), "helloworld");
    }

    #[test]
    fn exact_title_artist_and_close_duration_is_confident() {
        let candidate = SearchSongItem {
            id: 123,
            name: "合唱团, 阴郁的夜晚".into(),
            artists: vec![SearchArtist {
                id: 1,
                name: "V是兔子WishToday".into(),
            }],
            album: SearchAlbum {
                id: 1,
                name: "测试专辑".into(),
            },
            duration: 235_500,
        };
        assert_eq!(match_score(&local_song(), &candidate), 105);
    }

    #[test]
    fn empty_candidate_artist_does_not_score() {
        let candidate = SearchSongItem {
            id: 123,
            name: "合唱团，阴郁的夜晚".into(),
            artists: vec![SearchArtist {
                id: 0,
                name: String::new(),
            }],
            album: SearchAlbum {
                id: 1,
                name: "测试专辑".into(),
            },
            duration: 0,
        };
        assert_eq!(match_score(&local_song(), &candidate), 60);
        assert!(!artist_matches(&local_song(), &candidate));
    }

    #[test]
    fn different_artist_is_not_a_confident_match() {
        let candidate = SearchSongItem {
            id: 123,
            name: "合唱团，阴郁的夜晚".into(),
            artists: vec![SearchArtist {
                id: 2,
                name: "另一位歌手".into(),
            }],
            album: SearchAlbum {
                id: 1,
                name: "测试专辑".into(),
            },
            duration: 234_000,
        };
        assert_eq!(match_score(&local_song(), &candidate), 80);
        assert!(!artist_matches(&local_song(), &candidate));
    }

    #[tokio::test]
    async fn downloaded_song_metadata_is_inserted_then_updated_by_file_path() {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE songs (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, artist TEXT NOT NULL DEFAULT '', album TEXT NOT NULL DEFAULT '', genre TEXT NOT NULL DEFAULT '', year INTEGER DEFAULT 0, duration_ms INTEGER NOT NULL DEFAULT 0, file_path TEXT NOT NULL, lyrics_path TEXT NOT NULL DEFAULT '', cover_path TEXT NOT NULL DEFAULT '', filesize INTEGER NOT NULL DEFAULT 0, created_at DATETIME NOT NULL DEFAULT (datetime('now')), ncm_song_id INTEGER, metadata_source TEXT NOT NULL DEFAULT '', metadata_matched_at DATETIME)",
        )
        .execute(&db)
        .await
        .unwrap();

        let media_root =
            std::env::temp_dir().join(format!("radio-ncm-metadata-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(media_root.join("downloads"))
            .await
            .unwrap();
        let audio_path = media_root.join("downloads/test.mp3");
        let lyrics_path = media_root.join("downloads/test.lrc");
        tokio::fs::write(&audio_path, b"audio").await.unwrap();
        tokio::fs::write(&lyrics_path, b"[00:00]test")
            .await
            .unwrap();

        let mut detail = crate::services::ncm::types::SongDetailData {
            name: "测试歌曲".into(),
            id: 123,
            ar: vec![
                crate::services::ncm::types::SongArtist {
                    id: 1,
                    name: "歌手甲".into(),
                },
                crate::services::ncm::types::SongArtist {
                    id: 2,
                    name: "歌手乙".into(),
                },
            ],
            al: crate::services::ncm::types::SongAlbum {
                id: 9,
                name: "测试专辑".into(),
                pic_url: String::new(),
            },
            dt: 180_000,
        };

        let first_id = sync_downloaded_song(
            &db,
            &media_root,
            &audio_path,
            Some(&lyrics_path),
            &detail,
            5,
        )
        .await
        .unwrap();
        detail.name = "修正后的歌曲名".into();
        let second_id = sync_downloaded_song(&db, &media_root, &audio_path, None, &detail, 6)
            .await
            .unwrap();

        assert_eq!(first_id, second_id);
        let row: (i64, String, String, String, String, i64) = sqlx::query_as(
            "SELECT COUNT(*), title, artist, lyrics_path, metadata_source, filesize FROM songs",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(row.0, 1);
        assert_eq!(row.1, "修正后的歌曲名");
        assert_eq!(row.2, "歌手甲, 歌手乙");
        assert_eq!(row.3, "downloads/test.lrc");
        assert_eq!(row.4, "ncm_download");
        assert_eq!(row.5, 6);

        db.close().await;
        tokio::fs::remove_dir_all(&media_root).await.unwrap();
    }
}
