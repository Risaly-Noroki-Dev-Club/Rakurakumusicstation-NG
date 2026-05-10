use super::client::NcmClient;
use super::types::*;
use anyhow::Result;

pub async fn get_playlist_track_all(client: &NcmClient, id: i64) -> Result<Vec<SongDetailData>> {
    let req_json = serde_json::json!({
        "id": id,
        "limit": 100000,
        "offset": 0,
        "total": true
    })
    .to_string();

    let resp = client
        .eapi_request(
            "/api/playlist/track/all",
            "https://music.163.com/eapi/playlist/track/all",
            &req_json,
        )
        .await?;

    let data: PlaylistTrackAllData = serde_json::from_str(&resp)?;
    Ok(data.songs)
}
