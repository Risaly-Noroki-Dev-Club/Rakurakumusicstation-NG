use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::mpsc;

use crate::config::{
    AUDIO_CHUNK_SIZE, CHANNELS, CROSSFADE_SECONDS, MP3_BITRATE, SAMPLE_RATE,
    STATE_PUBLISH_INTERVAL_MS, SUPPORTED_FORMATS,
};
use crate::ring_buffer::RingBuffer;
use crate::types::{AudioCommand, FfmpegArgs, PlaybackState, TrackMetadata};

/// Build ffmpeg command line arguments for track playback.
pub fn build_ffmpeg_args(args: &FfmpegArgs, duration_ms: i64) -> Vec<String> {
    let mut cmd: Vec<String> = vec![
        "-nostdin".to_string(),
        "-re".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-i".to_string(),
        args.input_file.clone(),
        "-vn".to_string(),
        "-c:a".to_string(),
        "libmp3lame".to_string(),
        "-b:a".to_string(),
        MP3_BITRATE.to_string(),
        "-ar".to_string(),
        SAMPLE_RATE.to_string(),
        "-ac".to_string(),
        CHANNELS.to_string(),
    ];

    if args.fade_in {
        let afade = format!("afade=t=in:d={}:curve=tri", CROSSFADE_SECONDS);
        cmd.push("-af".to_string());
        cmd.push(afade);
    } else if duration_ms > (CROSSFADE_SECONDS as i64 * 2 * 1000) {
        let st = (duration_ms as f64 / 1000.0) - CROSSFADE_SECONDS as f64;
        let afade = format!("afade=t=out:st={:.3}:d={}:curve=tri", st, CROSSFADE_SECONDS);
        cmd.push("-af".to_string());
        cmd.push(afade);
    }

    if let Some(offset) = args.start_offset_secs {
        cmd.push("-ss".to_string());
        cmd.push(format!("{:.3}", offset));
    }

    cmd.push("-f".to_string());
    cmd.push("mp3".to_string());
    cmd.push("pipe:1".to_string());

    cmd
}

/// Spawn an ffmpeg process for track playback.
/// Returns the child process with stdout ready.
async fn spawn_ffmpeg(args: &FfmpegArgs, duration_ms: i64) -> io::Result<Child> {
    let ff_args = build_ffmpeg_args(args, duration_ms);
    Command::new("ffmpeg")
        .args(&ff_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
}

/// Preload state for crossfade. Tracks the next track being preloaded
/// including the ffmpeg process, its stdout, and track metadata.
struct PreloadState {
    stdout: ChildStdout,
    child: Child,
    track_idx: usize,
    duration_ms: i64,
}

/// Audio player that spawns ffmpeg, reads from pipe, pushes to ring buffer,
/// and handles crossfade between tracks.
pub struct Player {
    buffer: Arc<RingBuffer>,
    playlist: Arc<Mutex<Vec<String>>>,
    playlist_metadata: Arc<Mutex<Vec<TrackMetadata>>>,
    current_track: Arc<AtomicUsize>,
    cmd_rx: mpsc::UnboundedReceiver<AudioCommand>,
    state: Arc<Mutex<PlaybackState>>,
    stop_flag: Arc<AtomicBool>,
    media_path: String,
}

/// Handle for external control of the Player.
#[derive(Clone)]
pub struct PlayerHandle {
    cmd_tx: mpsc::UnboundedSender<AudioCommand>,
    state: Arc<Mutex<PlaybackState>>,
    stop_flag: Arc<AtomicBool>,
}

impl Player {
    /// Create a new player. Returns (Player, PlayerHandle).
    pub fn new(buffer: Arc<RingBuffer>, media_path: String) -> (Self, PlayerHandle) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(PlaybackState::default()));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let playlist = Arc::new(Mutex::new(Vec::new()));
        let playlist_metadata = Arc::new(Mutex::new(Vec::new()));
        let current_track = Arc::new(AtomicUsize::new(0));

        let player = Self {
            buffer,
            playlist,
            playlist_metadata,
            current_track,
            cmd_rx,
            state: Arc::clone(&state),
            stop_flag: Arc::clone(&stop_flag),
            media_path,
        };

        let handle = PlayerHandle {
            cmd_tx,
            state,
            stop_flag,
        };

        (player, handle)
    }

    /// Initialize playlist by scanning media_path.
    pub async fn init_playlist(&mut self) {
        let mut playlist = self.playlist.lock().unwrap();
        let mut metadata = self.playlist_metadata.lock().unwrap();

        playlist.clear();
        metadata.clear();

        let dir = Path::new(&self.media_path);
        if !dir.exists() || !dir.is_dir() {
            tracing::warn!("Media directory not found: {}", self.media_path);
            return;
        }

        let mut entries: Vec<_> = match std::fs::read_dir(dir) {
            Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
            Err(e) => {
                tracing::error!("Failed to read media directory: {}", e);
                return;
            }
        };

        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let filename = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let is_supported = SUPPORTED_FORMATS.iter().any(|fmt| {
                filename
                    .to_lowercase()
                    .ends_with(&format!(".{}", fmt))
            });

            if !is_supported {
                continue;
            }

            let full_path = path.to_string_lossy().to_string();

            let meta = crate::metadata::extract_metadata(&full_path, &self.media_path)
                .await
                .unwrap_or_else(|_| TrackMetadata {
                    filename: filename.clone(),
                    title: Path::new(&filename)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    file_path: filename.clone(),
                    ..Default::default()
                });

            playlist.push(filename);
            metadata.push(meta);
        }

        tracing::info!(
            "Playlist initialized: {} tracks from {}",
            playlist.len(),
            self.media_path
        );
    }

    /// Main playback loop.
    pub async fn run(&mut self) {
        loop {
            if self.stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let playlist_empty = {
                let pl = self.playlist.lock().unwrap();
                pl.is_empty()
            };

            if playlist_empty {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            let (track_filename, track_idx, duration_ms) =
                self.get_current_track_info();

            if track_filename.is_empty() {
                self.current_track.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            if !Path::new(&track_filename).exists() {
                tracing::error!("File not found: {}", track_filename);
                self.current_track.fetch_add(1, Ordering::Relaxed);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }

            tracing::info!(
                "Playing: {} ({}/{}) [{}s]",
                track_filename,
                track_idx + 1,
                self.playlist.lock().unwrap().len(),
                duration_ms / 1000
            );

            let _ = self
                .stream_track(&track_filename, track_idx, duration_ms)
                .await;
        }

        let mut state = self.state.lock().unwrap();
        state.song_id = 0;
        state.file_path.clear();
        state.position_ms = 0;
        state.duration_ms = 0;
        state.status = "stopped".to_string();
        state.total_bytes_sent = 0;

        tracing::info!("Player stopped");
    }

    /// Get the current track's absolute file path, index, and duration.
    fn get_current_track_info(&self) -> (String, usize, i64) {
        let pl = self.playlist.lock().unwrap();
        let pl_meta = self.playlist_metadata.lock().unwrap();
        let sz = pl.len();
        if sz == 0 {
            return (String::new(), 0, 0);
        }
        let idx = self.current_track.load(Ordering::Relaxed) % sz;
        let filename = pl[idx].clone();
        let full_path = Path::new(&self.media_path)
            .join(&filename)
            .to_string_lossy()
            .to_string();
        let duration = if idx < pl_meta.len() {
            (pl_meta[idx].duration_secs * 1000.0) as i64
        } else {
            0
        };
        (full_path, idx, duration)
    }

    /// Stream a single track with crossfade support.
    /// Returns Ok(()) when finished naturally, Err on fatal error.
    async fn stream_track(
        &mut self,
        initial_path: &str,
        mut track_idx: usize,
        mut duration_ms: i64,
    ) -> Result<(), anyhow::Error> {
        let file_path = initial_path.to_string();
        let ff_args = FfmpegArgs {
            input_file: file_path.clone(),
            fade_in: false,
            start_offset_secs: None,
        };

        let mut main_child = spawn_ffmpeg(&ff_args, duration_ms)
            .await
            .context("Failed to spawn main ffmpeg")?;
        let mut main_stdout = main_child
            .stdout
            .take()
            .context("Main ffmpeg stdout not available")?;

        let mut preload: Option<PreloadState> = None;
        let mut preload_read_buf = vec![0u8; AUDIO_CHUNK_SIZE];
        let mut preload_accum: Vec<u8> = Vec::new();
        let mut preload_triggered = false;

        let mut track_start = Instant::now();
        let mut track_start_system = chrono::Utc::now().timestamp_millis();
        let mut total_bytes_sent: u64 = 0;
        let mut main_buf = vec![0u8; AUDIO_CHUNK_SIZE];

        'stream: loop {
            tokio::select! {
                biased;

                // Commands (highest priority)
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            let should_break = self.handle_command(&cmd);
                            if should_break {
                                break 'stream;
                            }
                        }
                        None => break 'stream,
                    }
                }

                // Main audio data from ffmpeg
                result = main_stdout.read(&mut main_buf) => {
                    match result {
                        Ok(0) => {
                            // Main track ended naturally
                            if let Some(p) = preload.take() {
                                // Push accumulated preload data to ring buffer
                                if !preload_accum.is_empty() {
                                    tracing::info!(
                                        "[XFade] Drained {} bytes of preloaded audio",
                                        preload_accum.len()
                                    );
                                    self.buffer.push(&preload_accum);
                                    preload_accum.clear();
                                }

                                // Kill old main ffmpeg
                                let _ = main_child.kill().await;

                                // Switch preload to main
                                main_stdout = p.stdout;
                                main_child = p.child;
                                track_idx = p.track_idx;
                                duration_ms = p.duration_ms;

                                // Reset timing for the new track
                                track_start = Instant::now();
                                track_start_system = chrono::Utc::now().timestamp_millis();
                                total_bytes_sent = 0;
                                preload_triggered = false;

                                tracing::info!(
                                    "[XFade] Switched to next track (idx={})",
                                    track_idx
                                );

                                // Update playlist current_track
                                {
                                    let pl = self.playlist.lock().unwrap();
                                    let sz = pl.len();
                                    if sz > 0 {
                                        self.current_track
                                            .store(track_idx % sz, Ordering::Relaxed);
                                    }
                                }

                                continue 'stream;
                            }

                            // No preload active — end track normally
                            break 'stream;
                        }
                        Ok(n) => {
                            self.buffer.push(&main_buf[..n]);
                            total_bytes_sent += n as u64;
                        }
                        Err(e) => {
                            tracing::error!("Main ffmpeg read error: {}", e);
                            break 'stream;
                        }
                    }
                }

                // Preload audio data
                result = async {
                    match preload.as_mut() {
                        Some(p) => p.stdout.read(&mut preload_read_buf).await,
                        None => std::future::pending::<io::Result<usize>>().await,
                    }
                }, if preload.is_some() => {
                    match result {
                        Ok(0) => {
                            tracing::warn!("Preload ffmpeg ended early");
                            if let Some(mut p) = preload.take() {
                                let _ = p.child.kill().await;
                            }
                        }
                        Ok(n) => {
                            preload_accum.extend_from_slice(&preload_read_buf[..n]);
                        }
                        Err(e) => {
                            tracing::error!("Preload ffmpeg read error: {}", e);
                            if let Some(mut p) = preload.take() {
                                let _ = p.child.kill().await;
                            }
                        }
                    }
                }

                // Periodic state publish and crossfade trigger
                _ = tokio::time::sleep(Duration::from_millis(STATE_PUBLISH_INTERVAL_MS)) => {
                    // Update playback state for polling
                    self.publish_state(
                        track_idx,
                        duration_ms,
                        &track_start,
                        track_start_system,
                        total_bytes_sent,
                        preload_triggered,
                    );

                    // Check crossfade preload trigger
                    if !preload_triggered
                        && preload.is_none()
                        && duration_ms > (CROSSFADE_SECONDS as i64 * 1000)
                    {
                        let elapsed = track_start.elapsed().as_millis() as i64;
                        if elapsed >= duration_ms - (CROSSFADE_SECONDS as i64 * 1000) {
                            let (next_path, next_idx, next_duration) =
                                match self.peek_next_track() {
                                    Some(info) => info,
                                    None => continue,
                                };

                            let next_ff_args = FfmpegArgs {
                                input_file: next_path.clone(),
                                fade_in: true,
                                start_offset_secs: None,
                            };

                            match spawn_ffmpeg(&next_ff_args, next_duration).await {
                                Ok(mut child) => {
                                    if let Some(stdout) = child.stdout.take() {
                                        preload = Some(PreloadState {
                                            stdout,
                                            child,
                                            track_idx: next_idx,
                                            duration_ms: next_duration,
                                        });
                                        preload_triggered = true;
                                        preload_accum.clear();
                                        tracing::info!(
                                            "[XFade] Preloading next track: {}",
                                            next_path
                                        );
                                    } else {
                                        tracing::error!(
                                            "Preload ffmpeg stdout not available"
                                        );
                                        let _ = child.kill().await;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to spawn preload ffmpeg: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Cleanup main ffmpeg
        let _ = main_child.kill().await;

        // Cleanup preload if any
        if let Some(mut p) = preload.take() {
            let _ = p.child.kill().await;
        }

        // Advance to next track (unless we stopped/skipped)
        if !self.stop_flag.load(Ordering::Relaxed) {
            let pl = self.playlist.lock().unwrap();
            let sz = pl.len();
            if sz > 0 {
                let current = self.current_track.load(Ordering::Relaxed);
                self.current_track.store((current + 1) % sz, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Publish the current playback state to the shared state struct.
    fn publish_state(
        &self,
        track_idx: usize,
        duration_ms: i64,
        track_start: &Instant,
        track_start_system: i64,
        total_bytes_sent: u64,
        preload_triggered: bool,
    ) {
        let elapsed = track_start.elapsed();
        let position_ms = elapsed.as_millis() as i64;
        let clamped_pos = if duration_ms > 0 && position_ms > duration_ms {
            duration_ms
        } else {
            position_ms
        };

        let file_path_rel = {
            let pl = self.playlist.lock().unwrap();
            if track_idx < pl.len() {
                pl[track_idx].clone()
            } else {
                String::new()
            }
        };

        let mut state = self.state.lock().unwrap();
        state.song_id = track_idx as i64;
        state.file_path = file_path_rel;
        state.position_ms = clamped_pos;
        state.duration_ms = duration_ms;
        state.status = if preload_triggered {
            "crossfading".to_string()
        } else {
            "playing".to_string()
        };
        state.total_bytes_sent = total_bytes_sent;
        state.track_start_timestamp_ms = track_start_system;
    }

    /// Peek at the next track without advancing.
    fn peek_next_track(&self) -> Option<(String, usize, i64)> {
        let pl = self.playlist.lock().unwrap();
        let pl_meta = self.playlist_metadata.lock().unwrap();
        let sz = pl.len();
        if sz == 0 {
            return None;
        }
        let current = self.current_track.load(Ordering::Relaxed) % sz;
        let next_idx = (current + 1) % sz;
        let filename = pl[next_idx].clone();
        let full_path = Path::new(&self.media_path)
            .join(&filename)
            .to_string_lossy()
            .to_string();
        let duration = if next_idx < pl_meta.len() {
            (pl_meta[next_idx].duration_secs * 1000.0) as i64
        } else {
            0
        };

        if Path::new(&full_path).exists() {
            Some((full_path, next_idx, duration))
        } else {
            None
        }
    }

    /// Handle an audio command. Returns true if the current track should be skipped.
    fn handle_command(&self, cmd: &AudioCommand) -> bool {
        match cmd.cmd_type.as_str() {
            "skip" | "next" => {
                let pl = self.playlist.lock().unwrap();
                let sz = pl.len();
                if sz > 0 {
                    let current = self.current_track.load(Ordering::Relaxed);
                    self.current_track
                        .store((current + 1) % sz, Ordering::Relaxed);
                    tracing::info!("Skip command received");
                    return true;
                }
                false
            }
            "prev" => {
                let pl = self.playlist.lock().unwrap();
                let sz = pl.len();
                if sz > 0 {
                    let current = self.current_track.load(Ordering::Relaxed);
                    self.current_track
                        .store((current + sz - 1) % sz, Ordering::Relaxed);
                    tracing::info!("Prev command received");
                    return true;
                }
                false
            }
            "play" => {
                if let Some(ref fp) = cmd.file_path {
                    self.play_file(fp);
                    return true;
                }
                false
            }
            "stop" => {
                self.stop_flag.store(true, Ordering::Relaxed);
                tracing::info!("Stop command received");
                true
            }
            _ => false,
        }
    }

    /// Play a specific file by name (find in playlist or add it).
    fn play_file(&self, file_path: &str) {
        let mut pl = self.playlist.lock().unwrap();
        let mut pl_meta = self.playlist_metadata.lock().unwrap();

        for (i, p) in pl.iter().enumerate() {
            if p == file_path {
                self.current_track.store(i, Ordering::Relaxed);
                self.stop_flag.store(false, Ordering::Relaxed);
                return;
            }
        }

        pl.push(file_path.to_string());

        let meta = TrackMetadata {
            filename: Path::new(file_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            title: Path::new(file_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            file_path: file_path.to_string(),
            ..Default::default()
        };
        pl_meta.push(meta);

        self.current_track.store(pl.len() - 1, Ordering::Relaxed);
        self.stop_flag.store(false, Ordering::Relaxed);
    }
}

impl PlayerHandle {
    /// Send a command to the player.
    pub fn send_command(&self, cmd: AudioCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Get current playback state (snapshot).
    pub fn get_state(&self) -> PlaybackState {
        self.state.lock().unwrap().clone()
    }

    /// Request stop.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        let _ = self.cmd_tx.send(AudioCommand {
            cmd_type: "stop".to_string(),
            song_id: None,
            file_path: None,
        });
    }
}
