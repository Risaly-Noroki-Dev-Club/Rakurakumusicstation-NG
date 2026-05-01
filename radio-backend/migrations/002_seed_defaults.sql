-- 迁移 002：创建默认管理员账号（密码：admin123）
-- 仅对 SQLite 有用；若使用 PostgreSQL，请通过 API 创建管理员。

-- "admin123" 的默认管理员密码哈希值
-- 生成方式：argon2id("admin123") - 生产环境应使用真实的哈希值
INSERT OR IGNORE INTO users (username, password_hash, role)
VALUES (
    'admin',
    -- 这是 "admin123" 预先计算的 argon2id 哈希值
    -- 请在生产部署前修改此密码！
    '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
    'admin'
);

-- 插入一些默认的电台提示歌曲（可选）
INSERT OR IGNORE INTO songs (id, title, artist, album, duration_ms, file_path)
VALUES
    (1, 'Welcome to Rakuraku Radio', 'System', 'Station Messages', 5000, ''),
    (2, 'The Queue is Empty', 'System', 'Station Messages', 3000, '');
