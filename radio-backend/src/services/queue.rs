/// 队列管理器：共享的电台队列（FIFO），支持管理员覆盖操作。
use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::{QueueItem, QueueItemDisplay, SongSummary};
use sqlx::SqlitePool;
use std::sync::Arc;

/// 检查设备用户是否处于点歌冷却中。
pub async fn check_cooldown(
    db: &SqlitePool,
    device_user_id: i64,
    cooldown_secs: u64,
) -> Result<(), AppError> {
    if cooldown_secs == 0 {
        return Ok(());
    }

    let elapsed: Option<(i64,)> = sqlx::query_as(
        "SELECT strftime('%s', 'now') - strftime('%s', last_request_time) FROM user_requests WHERE device_user_id = ? AND last_request_time > datetime('now', '-' || ? || ' seconds')"
    )
    .bind(device_user_id)
    .bind(cooldown_secs as i64)
    .fetch_optional(db)
    .await?;

    if let Some((elapsed,)) = elapsed {
        let remaining = cooldown_secs.saturating_sub(elapsed.max(0) as u64);
        return Err(AppError::RateLimited(format!(
            "Cooldown active: please wait {} seconds before requesting another song",
            remaining
        )));
    }

    Ok(())
}

/// 检查设备用户是否超出队列提交的速率限制。
pub async fn check_rate_limit(
    db: &SqlitePool,
    device_user_id: i64,
    window_secs: u64,
    max_subs: usize,
) -> Result<bool, AppError> {
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(window_secs as i64);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM queue_items WHERE device_user_id = ? AND added_at > ?",
    )
    .bind(device_user_id)
    .bind(&cutoff_str)
    .fetch_one(db)
    .await?;

    Ok(count.0 as usize >= max_subs)
}

/// 获取当前队列大小（pending + playing 项目）。
pub async fn queue_size(db: &SqlitePool) -> Result<usize, AppError> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM queue_items WHERE status IN ('pending', 'playing')")
            .fetch_one(db)
            .await?;

    Ok(count.0 as usize)
}

/// 将歌曲添加到队列（尾部）。
pub async fn add_to_queue(
    state: &Arc<AppState>,
    song_id: i64,
    device_user_id: i64,
    display_name: &str,
) -> Result<i64, AppError> {
    let db = &state.db;
    let config = &state.config.queue;

    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(song_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    check_cooldown(db, device_user_id, config.request_cooldown_secs).await?;

    if check_rate_limit(
        db,
        device_user_id,
        config.rate_limit_window_secs,
        config.max_user_submissions,
    )
    .await?
    {
        return Err(AppError::RateLimited(format!(
            "You can only submit {} songs per {} seconds",
            config.max_user_submissions, config.rate_limit_window_secs
        )));
    }

    let current_size;
    let queue_item_id;
    {
        let _queue_guard = state.queue_sync.lock().await;

        current_size = queue_size(db).await?;
        if current_size >= config.max_size {
            return Err(AppError::BadRequest(format!(
                "Queue is full (max {} items)",
                config.max_size
            )));
        }

        let max_pos: Option<(i32,)> = sqlx::query_as(
            "SELECT MAX(position) FROM queue_items WHERE status IN ('pending', 'playing')",
        )
        .fetch_optional(db)
        .await?;

        let next_position = max_pos.map(|(p,)| p + 1).unwrap_or(0);

        let result = sqlx::query(
            "INSERT INTO queue_items (song_id, device_user_id, status, position) VALUES (?, ?, 'pending', ?)"
        )
        .bind(song_id)
        .bind(device_user_id)
        .bind(next_position)
        .execute(db)
        .await?;

        queue_item_id = result.last_insert_rowid();

        sqlx::query(
            "INSERT OR REPLACE INTO user_requests (device_user_id, last_request_time) VALUES (?, datetime('now'))"
        )
        .bind(device_user_id)
        .execute(db)
        .await?;

        // Push the request onto the engine queue so the player picks it up next.
        state
            .player_handle
            .enqueue_request(radio_engine::types::RequestedTrack {
                file_path: song.file_path.clone(),
                song_id: song.id,
                title: song.title.clone(),
                artist: song.artist.clone(),
                duration_ms: song.duration_ms,
            });
    }

    crate::websocket::broadcast(
        state,
        crate::models::WsMessage::QueueUpdate {
            action: "added".into(),
            song_title: Some(song.title.clone()),
            requested_by: None,
            queue_size: current_size + 1,
        },
    );

    tracing::info!(
        "Device '{}' added song '{}' to queue (item #{})",
        display_name,
        song.title,
        queue_item_id
    );

    Ok(queue_item_id)
}

/// 获取带歌曲详情的队列，按 position 排序。
pub async fn get_queue_display(db: &SqlitePool) -> Result<Vec<QueueItemDisplay>, AppError> {
    let items = sqlx::query_as::<_, QueueItem>(
        "SELECT * FROM queue_items WHERE status IN ('pending', 'playing') ORDER BY position ASC",
    )
    .fetch_all(db)
    .await?;

    let mut display_items = Vec::new();
    for item in items {
        let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
            .bind(item.song_id)
            .fetch_optional(db)
            .await?;

        let display_name =
            sqlx::query_as::<_, (String,)>("SELECT display_name FROM device_users WHERE id = ?")
                .bind(item.device_user_id)
                .fetch_optional(db)
                .await?
                .map(|(d,)| d)
                .unwrap_or_else(|| "unknown".into());

        display_items.push(QueueItemDisplay {
            id: item.id,
            song: song.map(SongSummary::from),
            requested_by: display_name,
            status: item.status,
            position: item.position,
            added_at: item.added_at,
        });
    }

    Ok(display_items)
}

/// 将队列项移动到新位置（仅限管理员）。
///
/// Uses a transaction to keep position updates atomic. Validates that
/// `new_position` falls within the current pending/playing queue range.
pub async fn move_queue_item(
    state: &Arc<AppState>,
    item_id: i64,
    new_position: i32,
) -> Result<(), AppError> {
    let db = &state.db;
    let _queue_guard = state.queue_sync.lock().await;

    if new_position < 0 {
        return Err(AppError::BadRequest(
            "Position must be non-negative".into(),
        ));
    }

    let item = sqlx::query_as::<_, QueueItem>("SELECT * FROM queue_items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Queue item not found".into()))?;

    if item.status != "pending" {
        return Err(AppError::BadRequest(
            "Only pending items can be moved".into(),
        ));
    }

    let old_position = item.position;

    if old_position == new_position {
        return Ok(());
    }

    let max_pos: i32 = sqlx::query_as::<_, (i32,)>(
        "SELECT COALESCE(MAX(position), 0) FROM queue_items WHERE status IN ('pending', 'playing')",
    )
    .fetch_one(db)
    .await?
    .0;

    if new_position > max_pos {
        return Err(AppError::BadRequest(format!(
            "Position {} exceeds max queue position {}",
            new_position, max_pos
        )));
    }

    let mut tx = db.begin().await?;

    if new_position < old_position {
        sqlx::query(
            "UPDATE queue_items SET position = position + 1 WHERE status IN ('pending', 'playing') AND position >= ? AND position < ?"
        )
        .bind(new_position)
        .bind(old_position)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE queue_items SET position = position - 1 WHERE status IN ('pending', 'playing') AND position > ? AND position <= ?"
        )
        .bind(old_position)
        .bind(new_position)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("UPDATE queue_items SET position = ? WHERE id = ?")
        .bind(new_position)
        .bind(item_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Keep the embedded engine request queue in sync with the DB order.
    rehydrate_engine_queue(state).await?;

    Ok(())
}

/// 删除队列项（仅限管理员）。
pub async fn remove_queue_item(state: &Arc<AppState>, item_id: i64) -> Result<(), AppError> {
    let db = &state.db;
    let _queue_guard = state.queue_sync.lock().await;
    let item = sqlx::query_as::<_, QueueItem>("SELECT * FROM queue_items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Queue item not found".into()))?;

    if item.status == "playing" {
        return Err(AppError::BadRequest(
            "Cannot remove the currently playing item; use skip instead".into(),
        ));
    }

    let removed_position = item.position;
    let removed_song_id = item.song_id;

    sqlx::query("UPDATE queue_items SET status = 'skipped' WHERE id = ?")
        .bind(item_id)
        .execute(db)
        .await?;

    sqlx::query(
        "UPDATE queue_items SET position = position - 1 WHERE status IN ('pending', 'playing') AND position > ?"
    )
    .bind(removed_position)
    .execute(db)
    .await?;

    // Pull it out of the engine request queue too, otherwise it'd still play.
    state
        .player_handle
        .remove_request_by_song_id(removed_song_id);

    Ok(())
}

/// 跳过当前正在播放的歌曲（仅限管理员）。
pub async fn skip_current(state: &Arc<AppState>) -> Result<(), AppError> {
    let db = &state.db;

    let playing = sqlx::query_as::<_, QueueItem>(
        "SELECT * FROM queue_items WHERE status = 'playing' ORDER BY position ASC LIMIT 1",
    )
    .fetch_optional(db)
    .await?;

    if let Some(item) = playing {
        sqlx::query(
            "UPDATE queue_items SET status = 'skipped', played_at = datetime('now') WHERE id = ?",
        )
        .bind(item.id)
        .execute(db)
        .await?;
    }

    let command = radio_engine::types::AudioCommand {
        cmd_type: radio_engine::types::AudioCommandType::Skip,
        song_id: None,
        file_path: None,
    };

    crate::websocket::publish_command(state, &command).await?;

    crate::websocket::broadcast(
        state,
        crate::models::WsMessage::Notice {
            message: "Admin skipped the current track".into(),
            level: "info".into(),
        },
    );

    Ok(())
}

/// 当音频引擎开始播放歌曲时调用。
pub async fn mark_playing(db: &SqlitePool, song_id: i64) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE queue_items SET status = 'played', played_at = datetime('now') WHERE status = 'playing'"
    )
    .execute(db)
    .await?;

    sqlx::query(
        "UPDATE queue_items SET status = 'playing' WHERE id = (SELECT id FROM queue_items WHERE song_id = ? AND status = 'pending' ORDER BY position ASC LIMIT 1)"
    )
    .bind(song_id)
    .execute(db)
    .await?;

    sqlx::query("INSERT INTO play_history (song_id, device_user_id) SELECT song_id, device_user_id FROM queue_items WHERE song_id = ? AND status = 'playing' ORDER BY id DESC LIMIT 1")
        .bind(song_id)
        .execute(db)
        .await?;

    Ok(())
}

/// 将数据库中所有 status='pending' 的队列项按 position 装回引擎请求队列。
///
/// 在服务启动时调用，让重启前用户已点的歌继续被播放。
pub async fn rehydrate_engine_queue(state: &Arc<AppState>) -> Result<(), AppError> {
    let rows = sqlx::query_as::<_, (String, i64, String, String, i64)>(
        "SELECT s.file_path, s.id, s.title, s.artist, s.duration_ms
         FROM queue_items q JOIN songs s ON s.id = q.song_id
         WHERE q.status = 'pending' ORDER BY q.position ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let tracks: Vec<radio_engine::types::RequestedTrack> = rows
        .into_iter()
        .map(|(file_path, song_id, title, artist, duration_ms)| {
            radio_engine::types::RequestedTrack {
                file_path,
                song_id,
                title,
                artist,
                duration_ms,
            }
        })
        .collect();

    let n = tracks.len();
    state.player_handle.replace_request_queue(tracks);
    if n > 0 {
        tracing::info!(
            "Rehydrated engine request queue with {} pending track(s)",
            n
        );
    }
    Ok(())
}

/// 获取队列头部（下一首要播放的歌曲）。
#[allow(dead_code)]
pub async fn get_next_song(db: &SqlitePool) -> Result<Option<crate::models::Song>, AppError> {
    let item = sqlx::query_as::<_, QueueItem>(
        "SELECT * FROM queue_items WHERE status = 'pending' ORDER BY position ASC LIMIT 1",
    )
    .fetch_optional(db)
    .await?;

    match item {
        Some(item) => {
            let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
                .bind(item.song_id)
                .fetch_optional(db)
                .await?;
            Ok(song)
        }
        None => Ok(None),
    }
}

/// 获取最近的播放历史。
pub async fn get_history(db: &SqlitePool, limit: i64) -> Result<Vec<serde_json::Value>, AppError> {
    let history = sqlx::query_as::<_, crate::models::PlayHistory>(
        "SELECT * FROM play_history ORDER BY played_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(db)
    .await?;

    let mut result = Vec::new();
    for h in history {
        let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
            .bind(h.song_id)
            .fetch_optional(db)
            .await?;

        let display_name = match h.device_user_id {
            Some(uid) => {
                sqlx::query_as::<_, (String,)>("SELECT display_name FROM device_users WHERE id = ?")
                    .bind(uid)
                    .fetch_optional(db)
                    .await?
                    .map(|(d,)| d)
                    .unwrap_or_else(|| "unknown".into())
            }
            None => "system".to_string(),
        };

        result.push(serde_json::json!({
            "id": h.id,
            "song": song.map(SongSummary::from),
            "requested_by": display_name,
            "played_at": h.played_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }));
    }

    Ok(result)
}
