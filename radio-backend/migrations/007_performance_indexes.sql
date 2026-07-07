-- Performance indexes for hot query paths

-- Used by playback_snapshot.rs:125 and queue.rs:396 WHERE file_path = ?
CREATE INDEX IF NOT EXISTS idx_songs_file_path ON songs(file_path);

-- Used by check_rate_limit() WHERE device_user_id = ? AND added_at > ?
CREATE INDEX IF NOT EXISTS idx_queue_user_id ON queue_items(device_user_id);

-- Used by get_history() ORDER BY played_at DESC
CREATE INDEX IF NOT EXISTS idx_history_played_at ON play_history(played_at DESC);
