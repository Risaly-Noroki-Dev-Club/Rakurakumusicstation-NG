use super::client::NcmClient;
use super::types::*;
use anyhow::{Context, Result};
use std::time::Duration;

pub async fn get_playlist_track_all(client: &NcmClient, id: i64) -> Result<Vec<SongDetailData>> {
    // `/eapi/playlist/track/all` has been removed upstream. Fetch the playlist's
    // stable track IDs first, then resolve those IDs through the song detail API.
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?
        .post("https://music.163.com/api/v6/playlist/detail")
        .header("Referer", "https://music.163.com/")
        .header("User-Agent", "Mozilla/5.0")
        .header("Cookie", client.build_cookie_header())
        .form(&[
            ("id", id.to_string()),
            ("n", "100000".into()),
            ("s", "8".into()),
        ])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let data: PlaylistDetailData = serde_json::from_str(&response)
        .with_context(|| format!("解析网易云歌单详情失败: {}", response_excerpt(&response)))?;
    if data.code != 200 {
        anyhow::bail!("网易云歌单详情返回 code={}", data.code);
    }

    let ids = data
        .playlist
        .track_ids
        .into_iter()
        .map(|track| track.id)
        .filter(|id| *id > 0)
        .collect::<Vec<_>>();
    let mut songs = Vec::with_capacity(ids.len());
    // Keep the public detail URL comfortably below common proxy URL limits.
    for chunk in ids.chunks(200) {
        songs.extend(super::api::get_song_detail(client, chunk).await?);
    }
    Ok(songs)
}

fn response_excerpt(response: &str) -> String {
    response
        .trim()
        .chars()
        .take(240)
        .collect::<String>()
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::PlaylistDetailData;

    #[test]
    fn parses_playlist_track_ids() {
        let data: PlaylistDetailData =
            serde_json::from_str(r#"{"code":200,"playlist":{"trackIds":[{"id":123},{"id":456}]}}"#)
                .unwrap();
        assert_eq!(
            data.playlist
                .track_ids
                .into_iter()
                .map(|track| track.id)
                .collect::<Vec<_>>(),
            vec![123, 456]
        );
    }
}
