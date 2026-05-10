-- 迁移 005：添加用户点歌冷却时间表

CREATE TABLE IF NOT EXISTS user_requests (
    device_user_id      INTEGER PRIMARY KEY REFERENCES device_users(id) ON DELETE CASCADE,
    last_request_time   DATETIME NOT NULL DEFAULT (datetime('now'))
);
