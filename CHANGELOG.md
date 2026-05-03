# 更新日志

## v2.2.0 — 2026-05-03

### Bug 修复

- **修复管理面板「提权」和「删除歌曲」操作失败** — `AdminView.vue` 的 `authHeaders()` 在 token 为空时不再发送 `Authorization: Bearer null`（之前会导致后端 401），改为仅在 token 存在时才包含 Authorization 请求头。
- **修复删除占位歌曲可能失败** — `delete_song` 增加 `file_path` 为空字符串时的守卫逻辑，避免路径拼接异常。
- **统一路由参数语法** — 管理面板路由 `{id}` → `:id`，与项目其他路由格式保持一致。

### 新功能

- **普通用户上传歌曲** — `POST /api/songs/upload`（需登录），不限管理员。曲库页面新增「上传歌曲」面板，支持选择 MP3/FLAC/WAV/OGG/M4A/AAC 文件（上限 100 MB）。
- **单曲下载** — `GET /api/songs/:id/download`（需登录），返回带 `Content-Disposition: attachment` 的音频文件流。曲库列表中每首歌曲旁新增 ⬇️ 下载按钮，点击即可保存到本地。
- **用户个人网易云账号** — 新增 `user_ncm` 数据库表，支持每个用户独立配置自己的网易云 Cookie 或手机号+密码。
  - `GET /api/ncm` — 查看当前用户的网易云状态
  - `POST /api/ncm` — 保存当前用户的网易云凭据
  - `POST /api/ncm/test` — 测试当前用户的网易云登录
  - 曲库页面新增「我的网易云账号」面板，普通用户可在其中配置自己的网易云账号用于下载 VIP 歌曲。

### 数据库变更

- 新增迁移 `002_user_ncm.sql`，创建 `user_ncm` 表（自动执行）。
