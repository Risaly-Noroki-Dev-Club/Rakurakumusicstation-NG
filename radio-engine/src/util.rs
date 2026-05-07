use std::path::{Path, PathBuf};

/// Resolve a media path: use absolute paths as-is, join relative paths against `media_root`.
pub fn resolve_media_path(path: impl AsRef<Path>, media_root: impl AsRef<Path>) -> PathBuf {
    let p = path.as_ref();
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        media_root.as_ref().join(p)
    }
}

/// Convert a path to be relative to `media_root`.
/// If the path is not under `media_root`, returns it unchanged.
pub fn relativize_media_path(path: impl AsRef<Path>, media_root: impl AsRef<Path>) -> String {
    let p = path.as_ref();
    let root = media_root.as_ref();
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().to_string()
}

/// Recursively scan a directory for supported audio files.
/// Returns a Vec of (absolute_path, relative_to_root) tuples, sorted by relative path.
pub fn scan_media_dir(
    dir: &Path,
    root: &Path,
    supported: &[&str],
) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(scan_media_dir(&path, root, supported));
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if supported.iter().any(|f| *f == ext_lower) {
                    let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
                    files.push((path, rel));
                }
            }
        }
    }
    files.sort_by(|a, b| a.1.cmp(&b.1));
    files
}
