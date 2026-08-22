use serde::Deserialize;

// ─── 搜索 ──────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SearchSongData {
    pub result: SearchSongResult,
    pub code: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchSongResult {
    pub songs: Vec<SearchSongItem>,
    #[serde(rename = "songCount")]
    pub song_count: i32,
    #[serde(rename = "hasMore", default)]
    pub has_more: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchSongItem {
    pub id: i64,
    pub name: String,
    pub artists: Vec<SearchArtist>,
    pub album: SearchAlbum,
    #[serde(default)]
    pub duration: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchArtist {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchAlbum {
    pub id: i64,
    pub name: String,
}

// ─── 歌曲 URL ──────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SongsURLData {
    pub data: Vec<SongURLData>,
    pub code: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SongURLData {
    pub id: i64,
    pub url: Option<String>,
    #[serde(default)]
    pub br: i64,
    #[serde(default)]
    pub size: i64,
    pub md5: Option<String>,
    pub code: i32,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
}

// ─── 歌曲详情 ──────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SongsDetailData {
    pub songs: Vec<SongDetailData>,
    pub code: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SongDetailData {
    pub name: String,
    pub id: i64,
    #[serde(alias = "artists")]
    pub ar: Vec<SongArtist>,
    #[serde(alias = "album")]
    pub al: SongAlbum,
    #[serde(alias = "duration")]
    pub dt: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SongArtist {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SongAlbum {
    pub id: i64,
    pub name: String,
    #[serde(rename = "picUrl")]
    pub pic_url: String,
}

// ─── 歌词 ──────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SongLyricData {
    pub lrc: Option<LyricContent>,
    pub tlyric: Option<LyricContent>,
    pub code: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LyricContent {
    pub lyric: String,
    pub version: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaylistDetailData {
    pub playlist: PlaylistDetail,
    pub code: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaylistDetail {
    #[serde(rename = "trackIds")]
    pub track_ids: Vec<PlaylistTrackId>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaylistTrackId {
    pub id: i64,
}

#[cfg(test)]
mod tests {
    use super::SongsURLData;

    #[test]
    fn unavailable_song_url_accepts_null_fields() {
        let response: SongsURLData = serde_json::from_str(
            r#"{"code":200,"data":[{"id":186016,"url":null,"br":0,"size":0,"md5":null,"code":404,"type":null}]}"#,
        )
        .unwrap();
        assert_eq!(response.data.len(), 1);
        assert!(response.data[0].url.is_none());
        assert!(response.data[0].md5.is_none());
        assert!(response.data[0].file_type.is_none());
    }
}
