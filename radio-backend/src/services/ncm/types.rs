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
    pub song_count: i32,
    #[serde(default)]
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
    pub url: String,
    pub br: i64,
    pub size: i64,
    pub md5: String,
    pub code: i32,
    #[serde(rename = "type")]
    pub file_type: String,
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
    pub ar: Vec<SongArtist>,
    pub al: SongAlbum,
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
