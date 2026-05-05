use serde::{Deserialize, Serialize};

/// Track metadata (equivalent to C++ TrackMetadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub filename: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: String,
    pub track_number: String,
    pub duration_secs: f64,
    pub cover_art: Vec<u8>,
    pub lyrics: String,
    pub file_path: String,
}

impl Default for TrackMetadata {
    fn default() -> Self {
        Self {
            filename: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            genre: String::new(),
            year: String::new(),
            track_number: String::new(),
            duration_secs: 0.0,
            cover_art: Vec::new(),
            lyrics: String::new(),
            file_path: String::new(),
        }
    }
}

impl TrackMetadata {
    /// Get display name (artist - title if available, otherwise filename)
    pub fn get_display_name(&self) -> String {
        if !self.artist.is_empty() && !self.title.is_empty() {
            format!("{} - {}", self.artist, self.title)
        } else if !self.title.is_empty() {
            self.title.clone()
        } else {
            self.filename.clone()
        }
    }
}

/// Playback state (published every 500ms, equivalent to C++ PlaybackState)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub song_id: i64,
    pub file_path: String,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub status: String,
    pub total_bytes_sent: u64,
    pub track_start_timestamp_ms: i64,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            song_id: 0,
            file_path: String::new(),
            position_ms: 0,
            duration_ms: 0,
            status: "stopped".to_string(),
            total_bytes_sent: 0,
            track_start_timestamp_ms: 0,
        }
    }
}

/// Command sent to the audio engine (from backend/frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCommand {
    #[serde(rename = "type")]
    pub cmd_type: String,
    pub song_id: Option<i64>,
    pub file_path: Option<String>,
}

/// FFmpeg arguments for a single track playback
#[derive(Debug, Clone)]
pub struct FfmpegArgs {
    pub input_file: String,
    pub fade_in: bool,
    pub start_offset_secs: Option<f64>,
}
