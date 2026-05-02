/// 队列管理器：共享的电台队列（FIFO），支持管理员覆盖操作。
///
/// 关键行为：
/// - 普通用户只能追加到队尾。
/// - 管理员可以移动、删除、跳过任意项目。
/// - 队列有大小上限和每用户速率限制。
/// - 出队时（歌曲被选中播放），状态变为 pending→playing→played。
/// - 通过 WebSocket 广播和 Redis pub/sub 发送通知。

use crate::db::AppState;
use crate::error::AppError;
use crate::models::{QueueEvent, QueueItem, QueueItemDisplay, SongSummary};
use sqlx::SqlitePool;
use std::sync::Arc;

/// 检查用户是否超出队列提交的速率限制。
pub async fn check_rate_limit(db: &SqlitePool, user_id: i64, window_secs: u64, max_subs: usize) -> Result<bool, AppError> {
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(window_secs as i64);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM queue_items WHERE user_id = ? AND added_at > ?"
    )
    .bind(user_id)
    .bind(&cutoff_str)
    .fetch_one(db)
    .await?;

    Ok(count.0 as usize >= max_subs)
}

/// 获取当前队列大小（pending + playing 项目）。
pub async fn queue_size(db: &SqlitePool) -> Result<usize, AppError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM queue_items WHERE status IN ('pending', 'playing')"
    )
    .fetch_one(db)
    .await?;

    Ok(count.0 as usize)
}

/// 将歌曲添加到队列（尾部）。由已认证的未被封禁用户调用。
pub async fn add_to_queue(
    state: &Arc<AppState>,
    song_id: i64,
    user_id: i64,
    username: &str,
) -> Result<i64, AppError> {
    let db = &state.db;
    let config = &state.config.queue;

    // 检查歌曲是否存在
    let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
        .bind(song_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Song not found".into()))?;

    // 检查速率限制
    if check_rate_limit(db, user_id, config.rate_limit_window_secs, config.max_user_submissions).await? {
        return Err(AppError::RateLimited(format!(
            "You can only submit {} songs per {} seconds",
            config.max_user_submissions,
            config.rate_limit_window_secs
        )));
    }

    // 检查队列大小上限
    let current_size = queue_size(db).await?;
    if current_size >= config.max_size {
        return Err(AppError::BadRequest(format!(
            "Queue is full (max {} items)",
            config.max_size
        )));
    }

    // 获取下一个位置编号（当前最大 position + 1）
    let max_pos: Option<(i32,)> = sqlx::query_as(
        "SELECT MAX(position) FROM queue_items WHERE status IN ('pending', 'playing')"
    )
    .fetch_optional(db)
    .await?;

    let next_position = max_pos.map(|(p,)| p + 1).unwrap_or(0);

    // 插入队列项
    let result = sqlx::query(
        "INSERT INTO queue_items (song_id, user_id, status, position) VALUES (?, ?, 'pending', ?)"
    )
    .bind(song_id)
    .bind(user_id)
    .bind(next_position)
    .execute(db)
    .await?;

    let queue_item_id = result.last_insert_rowid();

    // 通过 WebSocket 广播队列更新
    crate::websocket::broadcast(state, crate::models::WsMessage::QueueUpdate {
        action: "added".into(),
        song_title: Some(song.title.clone()),
        requested_by: Some(username.to_string()),
        queue_size: current_size + 1,
    });

    // 通过 Redis 发布队列事件，以便 C++ 引擎感知
    if let Some(ref mut conn) = state.redis_conn.clone() {
        let queue_json = serde_json::to_string(&QueueEvent {
            event_type: "added".into(),
            song_id: Some(song_id),
            file_path: Some(song.file_path.clone()),
        })
        .unwrap_or_default();

        let _ = redis::cmd("PUBLISH")
            .arg(&state.config.redis.queue_channel)
            .arg(&queue_json)
            .query_async::<_, ()>(conn)
            .await;
    }

    tracing::info!(
        "User '{}' added song '{}' to queue (item #{})",
        username,
        song.title,
        queue_item_id
    );

    Ok(queue_item_id)
}

/// 获取带歌曲详情的队列，按 position 排序。
pub async fn get_queue_display(db: &SqlitePool) -> Result<Vec<QueueItemDisplay>, AppError> {
    let items = sqlx::query_as::<_, QueueItem>(
        "SELECT * FROM queue_items WHERE status IN ('pending', 'playing') ORDER BY position ASC"
    )
    .fetch_all(db)
    .await?;

    let mut display_items = Vec::new();
    for item in items {
        let song = sqlx::query_as::<_, crate::models::Song>("SELECT * FROM songs WHERE id = ?")
            .bind(item.song_id)
            .fetch_optional(db)
            .await?;

        let username = sqlx::query_as::<_, (String,)>("SELECT username FROM users WHERE id = ?")
            .bind(item.user_id)
            .fetch_optional(db)
            .await?
            .map(|(u,)| u)
            .unwrap_or_else(|| "unknown".into());

        display_items.push(QueueItemDisplay {
            id: item.id,
            song: song.map(SongSummary::from),
            requested_by: username,
            status: item.status,
            position: item.position,
            added_at: item.added_at,
        });
    }

    Ok(display_items)
}

/// 将队列项移动到新位置（仅限管理员）。
/// 移动后重新编号所有项目。
pub async fn move_queue_item(
    db: &SqlitePool,
    item_id: i64,
    new_position: i32,
) -> Result<(), AppError> {
    // 获取此项
    let item = sqlx::query_as::<_, QueueItem>("SELECT * FROM queue_items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Queue item not found".into()))?;

    if item.status != "pending" {
        return Err(AppError::BadRequest("Only pending items can be moved".into()));
    }

    let old_position = item.position;

    if old_position == new_position {
        return Ok(());
    }

    // 在旧位置和新位置之间移动项目
    if new_position < old_position {
        // 上移：将 [new_position, old_position-1] 范围内的项目向下移动 1 位
        sqlx::query(
            "UPDATE queue_items SET position = position + 1 WHERE status IN ('pending', 'playing') AND position >= ? AND position < ?"
        )
        .bind(new_position)
        .bind(old_position)
        .execute(db)
        .await?;
    } else {
        // 下移：将 [old_position+1, new_position] 范围内的项目向上移动 1 位
        sqlx::query(
            "UPDATE queue_items SET position = position - 1 WHERE status IN ('pending', 'playing') AND position > ? AND position <= ?"
        )
        .bind(old_position)
        .bind(new_position)
        .execute(db)
        .await?;
    }

    // 更新已移动项目的位置
    sqlx::query("UPDATE queue_items SET position = ? WHERE id = ?")
        .bind(new_position)
        .bind(item_id)
        .execute(db)
        .await?;

    Ok(())
}

/// 删除队列项（仅限管理员）。
pub async fn remove_queue_item(db: &SqlitePool, item_id: i64) -> Result<(), AppError> {
    let item = sqlx::query_as::<_, QueueItem>("SELECT * FROM queue_items WHERE id = ?")
        .bind(item_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Queue item not found".into()))?;

    if item.status == "playing" {
        return Err(AppError::BadRequest("Cannot remove the currently playing item; use skip instead".into()));
    }

    let removed_position = item.position;

    // 将状态设为 skipped 并将剩余项目上移
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

    Ok(())
}

/// 跳过当前正在播放的歌曲（仅限管理员）。
/// 通过 Redis 向 C++ 音频引擎发送 skip 命令。
pub async fn skip_current(
    state: &Arc<AppState>,
) -> Result<(), AppError> {
    let db = &state.db;

    // 将正在播放的项目标记为 skipped
    let playing = sqlx::query_as::<_, QueueItem>(
        "SELECT * FROM queue_items WHERE status = 'playing' ORDER BY position ASC LIMIT 1"
    )
    .fetch_optional(db)
    .await?;

    if let Some(item) = playing {
        sqlx::query("UPDATE queue_items SET status = 'skipped', played_at = datetime('now') WHERE id = ?")
            .bind(item.id)
            .execute(db)
            .await?;
    }

    // 通过 Redis 向 C++ 音频引擎发送 skip 命令
    let command = crate::models::AudioCommand {
        cmd_type: "skip".into(),
        song_id: None,
        file_path: None,
    };

    crate::websocket::publish_command(state, &command).await?;

    // 广播通知
    crate::websocket::broadcast(state, crate::models::WsMessage::Notice {
        message: "Admin skipped the current track".into(),
        level: "info".into(),
    });

    Ok(())
}

/// 当 C++ 引擎开始播放歌曲时调用（通常通过 next 事件）。
pub async fn mark_playing(
    db: &SqlitePool,
    song_id: i64,
) -> Result<(), AppError> {
    // 将之前的 playing 标记为 played
    sqlx::query(
        "UPDATE queue_items SET status = 'played', played_at = datetime('now') WHERE status = 'playing'"
    )
    .execute(db)
    .await?;

    // 将新项目标记为 playing
    sqlx::query(
        "UPDATE queue_items SET status = 'playing' WHERE song_id = ? AND status = 'pending' ORDER BY position ASC LIMIT 1"
    )
    .bind(song_id)
    .execute(db)
    .await?;

    // 记录到播放历史
    sqlx::query("INSERT INTO play_history (song_id, user_id) SELECT song_id, user_id FROM queue_items WHERE song_id = ? AND status = 'playing' ORDER BY id DESC LIMIT 1")
        .bind(song_id)
        .execute(db)
        .await?;

    Ok(())
}

/// 获取队列头部（下一首要播放的歌曲）。
pub async fn get_next_song(db: &SqlitePool) -> Result<Option<crate::models::Song>, AppError> {
    let item = sqlx::query_as::<_, QueueItem>(
        "SELECT * FROM queue_items WHERE status = 'pending' ORDER BY position ASC LIMIT 1"
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
        "SELECT * FROM play_history ORDER BY played_at DESC LIMIT ?"
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

        let username = match h.user_id {
            Some(uid) => sqlx::query_as::<_, (String,)>("SELECT username FROM users WHERE id = ?")
                .bind(uid)
                .fetch_optional(db)
                .await?
                .map(|(u,)| u)
                .unwrap_or_else(|| "unknown".into()),
            None => "system".to_string(),
        };

        result.push(serde_json::json!({
            "id": h.id,
            "song": song.map(SongSummary::from),
            "requested_by": username,
            "played_at": h.played_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }));
    }

    Ok(result)
}
