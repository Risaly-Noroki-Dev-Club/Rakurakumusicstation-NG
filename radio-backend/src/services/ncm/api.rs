use super::client::NcmClient;
use super::types::*;
use anyhow::Result;

pub async fn search_song(client: &NcmClient, keyword: &str, limit: i32) -> Result<Vec<SearchSongItem>> {
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

    let data: SearchSongData = serde_json::from_str(&resp)?;
    Ok(data.result.songs)
}

pub async fn get_song_url(client: &NcmClient, ids: &[i64], level: &str) -> Result<Vec<SongURLData>> {
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

    let data: SongsURLData = serde_json::from_str(&resp)?;
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

    let data: SongsDetailData = serde_json::from_str(&resp)?;
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

    let data: SongLyricData = serde_json::from_str(&resp)?;
    Ok(data.lrc.map(|l| l.lyric))
}
