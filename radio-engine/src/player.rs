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

/// Mutable state for a single track's streaming loop.
struct StreamState {
    track_idx: usize,
    duration_ms: i64,
    track_start: Instant,
    track_start_system: i64,
    total_bytes_sent: u64,
}

/// Audio player that spawns ffmpeg, reads from pipe, pushes to ring buffer,
/// and handles crossfade between tracks.
pub struct Player {
    buffer: Arc<RingBuffer>,
    play_queue: Arc<Mutex<Vec<String>>>,
    play_queue_metadata: Arc<Mutex<Vec<TrackMetadata>>>,
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
        let play_queue = Arc::new(Mutex::new(Vec::new()));
        let play_queue_metadata = Arc::new(Mutex::new(Vec::new()));
        let current_track = Arc::new(AtomicUsize::new(0));

        let player = Self {
            buffer,
            play_queue,
            play_queue_metadata,
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

    /// Initialize play queue by recursively scanning media_path.
    pub async fn init_play_queue(&mut self) {
        let dir = Path::new(&self.media_path);
        if !dir.exists() || !dir.is_dir() {
            tracing::warn!("Media directory not found: {}", self.media_path);
            return;
        }

        let files = crate::util::scan_media_dir(dir, dir, SUPPORTED_FORMATS);

        let mut new_queue = Vec::new();
        let mut new_metadata = Vec::new();

        for (full_path, rel_path) in files {
            let filename = rel_path.clone();

            let meta = crate::metadata::extract_metadata(&rel_path, &self.media_path)
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

            new_queue.push(filename);
            new_metadata.push(meta);
        }

        let mut queue = self.play_queue.lock().unwrap();
        let mut metadata = self.play_queue_metadata.lock().unwrap();

        queue.clear();
        metadata.clear();
        queue.extend(new_queue);
        metadata.extend(new_metadata);

        tracing::info!(
            "Play queue initialized: {} tracks from {}",
            queue.len(),
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
                let pl = self.play_queue.lock().unwrap();
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
                self.play_queue.lock().unwrap().len(),
                duration_ms / 1000
            );

            let _ = self
                .stream_track(&track_filename, track_idx, duration_ms)
                .await;
        }

        let mut state = self.state.lock().unwrap();
        state.playlist_index = 0;
        state.file_path.clear();
        state.position_ms = 0;
        state.duration_ms = 0;
        state.status = crate::types::PlaybackStatus::Stopped;
        state.total_bytes_sent = 0;

        tracing::info!("Player stopped");
    }

    /// Get the current track's absolute file path, index, and duration.
    fn get_current_track_info(&self) -> (String, usize, i64) {
        let pl = self.play_queue.lock().unwrap();
        let pl_meta = self.play_queue_metadata.lock().unwrap();
        let sz = pl.len();
        if sz == 0 {
            return (String::new(), 0, 0);
        }
        let idx = self.current_track.load(Ordering::Relaxed) % sz;
        let rel_path = pl[idx].clone();
        let full_path = crate::util::resolve_media_path(&rel_path, &self.media_path)
            .to_string_lossy()
            .to_string();
        let duration = if idx < pl_meta.len() {
            pl_meta[idx].duration_ms
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
        track_idx: usize,
        duration_ms: i64,
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

        // Spawn a dedicated task to read from ffmpeg stdout so that
        // `main_stdout.read()` never blocks the state-publish timer.
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        
        let ffmpeg_task = tokio::spawn(async move {
            let mut buf = vec![0u8; AUDIO_CHUNK_SIZE];
            loop {
                match main_stdout.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if audio_tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            
            // 确保ffmpeg子进程被正确清理
            match main_child.kill().await {
                Ok(_) => tracing::debug!("FFmpeg process terminated cleanly"),
                Err(e) => tracing::warn!("Failed to kill FFmpeg process: {}", e),
            }
        });

        let mut state = StreamState {
            track_idx,
            duration_ms,
            track_start: Instant::now(),
            track_start_system: chrono::Utc::now().timestamp_millis(),
            total_bytes_sent: 0,
        };

        let mut last_publish = Instant::now();
        'stream: loop {
            if let Ok(cmd) = self.cmd_rx.try_recv() {
                if self.handle_command(&cmd) {
                    // 立即取消 ffmpeg 任务，触发 kill_on_drop 清理子进程
                    ffmpeg_task.abort();
                    break 'stream;
                }
            }

            match audio_rx.try_recv() {
                Ok(d) => {
                    self.buffer.push(&d);
                    state.total_bytes_sent += d.len() as u64;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // ffmpeg 已自然结束，等待任务完成
                    let _ = ffmpeg_task.await;
                    break 'stream;
                }
            }

            if last_publish.elapsed() >= Duration::from_millis(STATE_PUBLISH_INTERVAL_MS) {
                self.publish_state_from_stream(&state);
                last_publish = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Advance to next track (unless we stopped/skipped)
        if !self.stop_flag.load(Ordering::Relaxed) {
            let pl = self.play_queue.lock().unwrap();
            let sz = pl.len();
            if sz > 0 {
                let current = self.current_track.load(Ordering::Relaxed);
                self.current_track.store((current + 1) % sz, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Publish playback state using values from the stream loop.
    fn publish_state_from_stream(&self, state: &StreamState) {
        self.publish_state(
            state.track_idx,
            state.duration_ms,
            &state.track_start,
            state.track_start_system,
            state.total_bytes_sent,
            false,
        );
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
    // publish_state logging removed for production

    let file_path_rel = {
        let pl = self.play_queue.lock().unwrap();
        if track_idx < pl.len() {
            pl[track_idx].clone()
        } else {
            String::new()
        }
    };

        let mut state = self.state.lock().unwrap();
        state.playlist_index = track_idx as i64;
        state.file_path = file_path_rel;
        state.position_ms = clamped_pos;
        state.duration_ms = duration_ms;
        state.status = if preload_triggered {
            crate::types::PlaybackStatus::Crossfading
        } else {
            crate::types::PlaybackStatus::Playing
        };
        state.total_bytes_sent = total_bytes_sent;
        state.track_start_timestamp_ms = track_start_system;
    }

    /// Peek at the next track without advancing.
    fn peek_next_track(&self) -> Option<(String, usize, i64)> {
        let pl = self.play_queue.lock().unwrap();
        let pl_meta = self.play_queue_metadata.lock().unwrap();
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
            pl_meta[next_idx].duration_ms
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
        use crate::types::AudioCommandType::*;
        match cmd.cmd_type {
            Skip | Next => {
                let pl = self.play_queue.lock().unwrap();
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
            Prev => {
                let pl = self.play_queue.lock().unwrap();
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
            Play => {
                if let Some(ref fp) = cmd.file_path {
                    self.play_file(fp);
                    return true;
                }
                false
            }
            Stop => {
                self.stop_flag.store(true, Ordering::Relaxed);
                tracing::info!("Stop command received");
                true
            }
        }
    }

    /// Play a specific file by name (find in playlist or add it).
    fn play_file(&self, file_path: &str) {
        let mut pl = self.play_queue.lock().unwrap();
        let mut pl_meta = self.play_queue_metadata.lock().unwrap();

        // Normalize to relative path for consistent storage
        let rel_path = crate::util::relativize_media_path(file_path, &self.media_path);

        for (i, p) in pl.iter().enumerate() {
            if p == &rel_path {
                self.current_track.store(i, Ordering::Relaxed);
                self.stop_flag.store(false, Ordering::Relaxed);
                return;
            }
        }

        pl.push(rel_path.clone());

        let meta = TrackMetadata {
            filename: Path::new(&rel_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            title: Path::new(&rel_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            file_path: rel_path,
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
            cmd_type: crate::types::AudioCommandType::Stop,
            song_id: None,
            file_path: None,
        });
    }
}
