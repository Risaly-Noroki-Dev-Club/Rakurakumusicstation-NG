-- 迁移 002：插入默认电台提示歌曲
-- 管理员账户通过首次运行的 Web 设置向导创建，不再自动种子化。

-- 插入一些默认的电台提示歌曲（可选）
INSERT OR IGNORE INTO songs (id, title, artist, album, duration_ms, file_path)
VALUES
    (1, 'Welcome to Rakuraku Radio', 'System', 'Station Messages', 5000, ''),
    (2, 'The Queue is Empty', 'System', 'Station Messages', 3000, '');
