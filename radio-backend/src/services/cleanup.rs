//! 定期清理僵尸设备用户。

use sqlx::SqlitePool;

/// 不活跃判定窗口（天）。
const INACTIVE_DAYS: i64 = 30;

/// 清理超过 30 天不活跃、仍使用默认自动名字（`Listener-*`）、且非管理员的
/// 空设备用户。任何持久用户数据都会阻止清理，避免将“最近未点歌”误判为
/// “账户可以删除”。
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
           AND NOT EXISTS (
             SELECT 1 FROM playlists p
             WHERE p.device_user_id = device_users.id
           )
           AND NOT EXISTS (
             SELECT 1 FROM favorites f
             WHERE f.device_user_id = device_users.id
           )
           AND NOT EXISTS (
             SELECT 1 FROM user_ncm n
             WHERE n.device_user_id = device_users.id
           )
           AND NOT EXISTS (
             SELECT 1 FROM queue_items q
             WHERE q.device_user_id = device_users.id
           )
         RETURNING id",
    )
    .bind(INACTIVE_DAYS)
    .bind(INACTIVE_DAYS)
    .fetch_all(db)
    .await?;

    Ok(deleted.len())
}

#[cfg(test)]
mod tests {
    use super::prune_inactive_default_users;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn insert_old_default_user(db: &sqlx::SqlitePool, name: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO device_users (device_token, display_name, created_at)
             VALUES (?, ?, datetime('now', '-40 days'))
             RETURNING id",
        )
        .bind(format!("token-{name}"))
        .bind(name)
        .fetch_one(db)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn only_prunes_empty_default_users() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&db).await.unwrap();

        let empty_user = insert_old_default_user(&db, "Listener-empty").await;
        let playlist_user = insert_old_default_user(&db, "Listener-playlist").await;
        let favorite_user = insert_old_default_user(&db, "Listener-favorite").await;
        let ncm_user = insert_old_default_user(&db, "Listener-ncm").await;
        let queue_user = insert_old_default_user(&db, "Listener-queue").await;

        let song_id: i64 = sqlx::query_scalar(
            "INSERT INTO songs (title, file_path) VALUES ('test', 'test.mp3') RETURNING id",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO playlists (device_user_id, name) VALUES (?, 'saved')")
            .bind(playlist_user)
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO favorites (device_user_id, song_id) VALUES (?, ?)")
            .bind(favorite_user)
            .bind(song_id)
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO user_ncm (device_user_id, ncm_cookie) VALUES (?, 'MUSIC_U=test')")
            .bind(ncm_user)
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO queue_items (song_id, device_user_id) VALUES (?, ?)")
            .bind(song_id)
            .bind(queue_user)
            .execute(&db)
            .await
            .unwrap();

        assert_eq!(prune_inactive_default_users(&db).await.unwrap(), 1);

        let remaining: Vec<i64> = sqlx::query_scalar("SELECT id FROM device_users ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();
        assert!(!remaining.contains(&empty_user));
        assert_eq!(remaining.len(), 4);
    }
}
