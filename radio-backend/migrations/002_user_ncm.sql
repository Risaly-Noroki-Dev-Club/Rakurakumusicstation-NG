-- 迁移 002：用户个人网易云音乐账号配置

CREATE TABLE IF NOT EXISTS user_ncm (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    ncm_cookie  TEXT NOT NULL DEFAULT '',
    ncm_phone   TEXT NOT NULL DEFAULT '',
    ncm_password TEXT NOT NULL DEFAULT '',
    created_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at  DATETIME NOT NULL DEFAULT (datetime('now'))
);
