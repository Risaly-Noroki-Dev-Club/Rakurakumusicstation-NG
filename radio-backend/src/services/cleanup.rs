//! 定期清理僵尸设备用户。

use sqlx::SqlitePool;

/// 不活跃判定窗口（天）。
const INACTIVE_DAYS: i64 = 30;

/// 清理超过 30 天不活跃、仍使用默认自动名字（`Listener-*`）、且非管理员的
/// 普通设备用户。
///
/// 级联删除依赖 SQLite 外键（db.rs `foreign_keys(true)`）：
/// queue_items / favorites / playlists / user_ncm / user_requests 一并清除。
/// 返回被删除的用户数。
pub async fn prune_inactive_default_users(db: &SqlitePool) -> anyhow::Result<usize> {
    let deleted = sqlx::query_scalar::<_, i64>(
        "DELETE FROM device_users
         WHERE role = 'user'
           AND display_name LIKE 'Listener-%'
           AND created_at < datetime('now', ? || ' days')
           AND NOT EXISTS (
             SELECT 1 FROM user_requests ur
             WHERE ur.device_user_id = device_users.id
               AND ur.last_request_time > datetime('now', ? || ' days')
           )
         RETURNING id",
    )
    .bind(INACTIVE_DAYS)
    .bind(INACTIVE_DAYS)
    .fetch_all(db)
    .await?;

    Ok(deleted.len())
}
