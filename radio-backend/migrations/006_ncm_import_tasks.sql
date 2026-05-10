-- 迁移 005：网易云歌单导入任务表

CREATE TABLE IF NOT EXISTS ncm_import_tasks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id     INTEGER NOT NULL,
    name        TEXT NOT NULL,
    artists     TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'queued', 'done', 'failed')),
    batch_id    TEXT NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at  DATETIME NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_ncm_import_tasks_status ON ncm_import_tasks(status);
CREATE INDEX IF NOT EXISTS idx_ncm_import_tasks_batch  ON ncm_import_tasks(batch_id);
