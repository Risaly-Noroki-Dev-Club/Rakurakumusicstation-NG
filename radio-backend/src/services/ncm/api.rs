use super::client::NcmClient;
use super::types::*;
use anyhow::{Context, Result};
use std::time::Duration;

fn anonymous_http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?)
}

/// 网易云公开搜索接口，不携带登录 Cookie。
pub async fn search_song_anonymous(keyword: &str, limit: i32) -> Result<Vec<SearchSongItem>> {
    let response = anonymous_http_client()?
        .post("https://music.163.com/api/search/get")
        .header("Referer", "https://music.163.com/")
        .header("User-Agent", "Mozilla/5.0")
        .form(&[
            ("s", keyword.to_string()),
            ("type", "1".to_string()),
            ("offset", "0".to_string()),
            ("limit", limit.to_string()),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<SearchSongData>()
        .await
        .with_context(|| "解析网易云匿名搜索结果失败")?;
    Ok(response.result.songs)
}

/// 网易云公开歌曲详情接口，不携带登录 Cookie。
pub async fn get_song_detail_anonymous(id: i64) -> Result<Option<SongDetailData>> {
    let ids = format!("[{}]", id);
    let response = anonymous_http_client()?
        .get("https://music.163.com/api/song/detail")
        .header("Referer", "https://music.163.com/")
        .header("User-Agent", "Mozilla/5.0")
        .query(&[("ids", ids)])
        .send()
        .await?
        .error_for_status()?
        .json::<SongsDetailData>()
        .await
        .with_context(|| "解析网易云匿名歌曲详情失败")?;
    Ok(response.songs.into_iter().next())
}

pub async fn search_song(
    client: &NcmClient,
    keyword: &str,
    limit: i32,
) -> Result<Vec<SearchSongItem>> {
    let req_json = serde_json::json!({
        "s": keyword,
        "offset": 0,
        "limit": limit
    })
    .to_string();

    let resp = client
        .eapi_request(
            "/api/v1/search/song/get",
            "https://music.163.com/eapi/v1/search/song/get",
            &req_json,
        )
        .await?;

    let data: SearchSongData =
        serde_json::from_str(&resp).with_context(|| "解析网易云搜索结果失败")?;
    Ok(data.result.songs)
}

pub async fn get_song_url(
    client: &NcmClient,
    ids: &[i64],
    level: &str,
) -> Result<Vec<SongURLData>> {
    let ids_json: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    let req_json = serde_json::json!({
        "encodeType": "mp3",
        "ids": serde_json::to_string(&ids_json)?,
        "level": level,
    })
    .to_string();

    let resp = client
        .eapi_request(
            "/api/song/enhance/player/url/v1",
            "https://music.163.com/eapi/song/enhance/player/url/v1",
            &req_json,
        )
        .await?;

    let data: SongsURLData =
        serde_json::from_str(&resp).with_context(|| "解析网易云下载链接失败")?;
    Ok(data.data)
}

pub async fn get_song_detail(client: &NcmClient, ids: &[i64]) -> Result<Vec<SongDetailData>> {
    let ids_arr: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| serde_json::json!({ "id": id }))
        .collect();
    let req_json = serde_json::json!({
        "c": serde_json::to_string(&ids_arr)?
    })
    .to_string();

    let resp = client
        .eapi_request(
            "/api/v3/song/detail",
            "https://music.163.com/eapi/v3/song/detail",
            &req_json,
        )
        .await?;

    let data: SongsDetailData =
        serde_json::from_str(&resp).with_context(|| "解析网易云歌曲详情失败")?;
    Ok(data.songs)
}

pub async fn get_song_lyric(client: &NcmClient, id: i64) -> Result<Option<String>> {
    let req_json = serde_json::json!({
        "id": id,
        "lv": -1,
        "kv": -1,
        "tv": -1,
        "yv": -1
    })
    .to_string();

    let resp = client
        .eapi_request(
            "/api/song/lyric",
            "https://music.163.com/eapi/song/lyric",
            &req_json,
        )
        .await?;

    let data: SongLyricData = serde_json::from_str(&resp).with_context(|| "解析网易云歌词失败")?;
    Ok(data.lrc.map(|l| l.lyric))
}
