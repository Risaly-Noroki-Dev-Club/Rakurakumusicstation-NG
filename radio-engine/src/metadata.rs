use std::path::Path;

use anyhow::Context;
use regex::Regex;

use crate::types::TrackMetadata;

/// Extract metadata from an audio file path.
/// Uses ffprobe for duration and lyrics, regex for artist-title from filename.
pub async fn extract_metadata(file_path: &str, media_root: &str) -> anyhow::Result<TrackMetadata> {
    let path = Path::new(file_path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    let full_path = if Path::new(file_path).is_absolute() {
        file_path.to_string()
    } else {
        Path::new(media_root).join(file_path).to_string_lossy().to_string()
    };

    let file_path_clone = full_path.clone();

    let stem = Path::new(&filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.clone());

    let (artist, title) = parse_artist_title(&stem);

    let duration_secs = get_duration(&full_path).await.unwrap_or(0.0);
    let lyrics = get_lyrics(&full_path).await.unwrap_or_default();

    Ok(TrackMetadata {
        filename,
        title,
        artist,
        album: String::new(),
        genre: String::new(),
        year: String::new(),
        track_number: String::new(),
        duration_secs,
        cover_art: Vec::new(),
        lyrics,
        file_path: file_path_clone,
    })
}

/// Get audio duration in seconds via ffprobe.
pub async fn get_duration(file_path: &str) -> anyhow::Result<f64> {
    let output = tokio::process::Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("csv=p=0")
        .arg(file_path)
        .output()
        .await
        .context("Failed to run ffprobe for duration")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe exited with non-zero status");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();
    if line.is_empty() {
        return Ok(0.0);
    }

    line.parse::<f64>().context("Failed to parse duration from ffprobe output")
}

/// Extract embedded lyrics via ffprobe.
pub async fn get_lyrics(file_path: &str) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format_tags")
        .arg("-of")
        .arg("default")
        .arg(file_path)
        .output()
        .await
        .context("Failed to run ffprobe for lyrics")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe exited with non-zero status");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if !line.starts_with("TAG:") {
            continue;
        }
        let tag = &line[4..];
        if let Some(eq_pos) = tag.find('=') {
            let key = &tag[..eq_pos];
            let value = &tag[eq_pos + 1..];

            let key_lower = key.to_lowercase();
            if key_lower == "lyrics"
                || key_lower == "unsyncedlyrics"
                || key_lower == "lyrics-eng"
                || key_lower == "lyrics-chi"
                || key_lower == "syncedlyrics"
                || key_lower.contains("lyrics")
            {
                return Ok(value.to_string());
            }
        }
    }

    Ok(String::new())
}

/// Parse "Artist - Title" from filename stem.
/// Returns (artist, title).
pub fn parse_artist_title(filename: &str) -> (String, String) {
    let re = Regex::new(r"^(.+?)\s*[-–—]\s*(.+)$").unwrap();
    if let Some(caps) = re.captures(filename) {
        let artist = caps.get(1).unwrap().as_str().trim().to_string();
        let title = caps.get(2).unwrap().as_str().trim().to_string();
        (artist, title)
    } else {
        (String::new(), filename.to_string())
    }
}

/// Check if a file has a supported audio format.
pub fn is_supported_format(filename: &str) -> bool {
    let path = Path::new(filename);
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        crate::config::SUPPORTED_FORMATS
            .iter()
            .any(|f| *f == ext_lower)
    } else {
        false
    }
}
