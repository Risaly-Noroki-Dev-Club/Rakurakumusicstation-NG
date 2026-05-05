-- 迁移 004：用基于设备的身份认证（通过 httpOnly Cookie）替换密码认证
-- 删除旧的 users 表以及引用它的所有表，然后重新创建它们。

PRAGMA foreign_keys = OFF;

-- ─── 删除引用旧的 users 表的所有表 ──────────────────────
DROP TABLE IF EXISTS user_ncm;
DROP TABLE IF EXISTS favorites;
DROP TABLE IF EXISTS admin_log;
DROP TABLE IF EXISTS play_history;
DROP TABLE IF EXISTS queue_items;
DROP TABLE IF EXISTS playlist_songs;
DROP TABLE IF EXISTS playlists;

-- ─── 删除旧的 users 表 ──────────────────────────────────
DROP TABLE IF EXISTS users;

-- ─── device_users：基于设备的身份。每个浏览器 / 设备有一个设备令牌 ──
CREATE TABLE device_users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_token    TEXT NOT NULL UNIQUE,     -- 64 字符随机 base64 令牌，存储在 httpOnly Cookie 中
    display_name    TEXT NOT NULL DEFAULT '', -- 用户可见的显示名称（可自定义）
    role            TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    banned_until    DATETIME,                -- NULL = 未被封禁，否则封禁至此时
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_device_token ON device_users(device_token);

-- ─── 重新创建引用 device_users 的表 ────────────────────

-- 歌单表：设备用户的个人歌单
CREATE TABLE playlists (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_user_id  INTEGER NOT NULL REFERENCES device_users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    is_public       INTEGER NOT NULL DEFAULT 0,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- 歌单项目表：歌单中的歌曲
CREATE TABLE playlist_songs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    playlist_id     INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    song_id         INTEGER NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    position        INTEGER NOT NULL DEFAULT 0,
    added_at        DATETIME NOT NULL DEFAULT (datetime('now')),
    UNIQUE(playlist_id, song_id)
);

-- 队列表：共享的电台队列（先进先出）
CREATE TABLE queue_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id         INTEGER NOT NULL REFERENCES songs(id) ON DELETE SET NULL,
    device_user_id  INTEGER NOT NULL REFERENCES device_users(id) ON DELETE SET NULL,
    status          TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'playing', 'played', 'skipped')),
    position        INTEGER NOT NULL DEFAULT 0,
    added_at        DATETIME NOT NULL DEFAULT (datetime('now')),
    played_at       DATETIME
);

CREATE INDEX IF NOT EXISTS idx_queue_status_pos ON queue_items(status, position);

-- 播放历史表：之前播放过的歌曲
CREATE TABLE play_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id         INTEGER NOT NULL REFERENCES songs(id) ON DELETE SET NULL,
    device_user_id  INTEGER REFERENCES device_users(id) ON DELETE SET NULL,
    played_at       DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- 管理员操作日志表
CREATE TABLE admin_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    admin_id        INTEGER NOT NULL REFERENCES device_users(id) ON DELETE CASCADE,
    action          TEXT NOT NULL,
    details         TEXT NOT NULL DEFAULT '',
    created_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- 设备用户收藏夹
CREATE TABLE favorites (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_user_id  INTEGER NOT NULL REFERENCES device_users(id) ON DELETE CASCADE,
    song_id         INTEGER REFERENCES songs(id) ON DELETE CASCADE,
    playlist_id     INTEGER REFERENCES playlists(id) ON DELETE CASCADE,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    CHECK (song_id IS NOT NULL OR playlist_id IS NOT NULL)
);

-- 设备的网易云音乐账号设置
CREATE TABLE user_ncm (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    device_user_id  INTEGER NOT NULL UNIQUE REFERENCES device_users(id) ON DELETE CASCADE,
    ncm_cookie      TEXT NOT NULL DEFAULT '',
    ncm_phone       TEXT NOT NULL DEFAULT '',
    ncm_password    TEXT NOT NULL DEFAULT '',
    created_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now'))
);

PRAGMA foreign_keys = ON;
