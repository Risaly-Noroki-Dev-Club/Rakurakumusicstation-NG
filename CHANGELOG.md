# 更新日志

## Unreleased

### 新功能

- **原生子路径部署** — `[server] base_path` 支持将后端挂载到 `/radio` 等子路径；API、WebSocket、音频流、静态前端、设备 Cookie path 和播放文件 URL 会同步使用该前缀。
- **子路径 PWA 支持** — 前端新增 `VITE_BASE_PATH` 构建前缀；Vue Router、manifest、service worker scope、PWA 图标与 API/WebSocket URL 都支持根路径和子路径。
- **PWA 图标补齐** — 新增 `icon-192.png` 和 `icon-512.png`，提升浏览器安装兼容性。
- **公开全站曲库** — 曲库页默认展示电台所有歌曲、总数和分页加载，搜索结果沿用同一列表体验。
- **真实深浅色主题** — 自动/浅色/深色会同步 Vuetify 主题和自定义 CSS 变量，自动模式跟随系统主题变化。
- **非线性动效** — 列表进入、迷你播放器、动态背景和主题切换使用强调曲线，并支持 reduced-motion 降级。

### 修复

- **修复 PWA 安装脆弱性** — service worker 预缓存改为逐项容错，导航请求改为 network-first，避免单个资源失败导致安装失败或长期使用旧页面。
- **修复图标对齐** — 统一 Vuetify/MDI 图标 `line-height` 与 flex 对齐，修正桌面 rail 导航和底部导航图标偏移。
- **改善网易云下载错误** — 空响应、非 JSON 响应、HTTP 下载失败和空文件会给出明确日志，避免只显示 `expected value at line 1 column 1`。
- **改善管理下载请求错误提示** — 前端下载启动不再假设响应一定是 JSON，反代错误页会显示 HTTP 状态和响应摘要。

---

## v3.1.0 — 2026-05-10

### 重大变更：原生 Rust 网易云下载（取代 music_dl.py）

- **移除 Python 依赖** — 删除 `music_dl.py` 子进程调用，网易云批量下载完全内置于 Rust 后端。
  - 新增 `radio-backend/src/services/ncm/` 模块，实现 Eapi 加密、HTTP 客户端、搜索、取 URL、下载、歌词获取全流程。
  - 参考 [Music163bot-Go](https://github.com/XiaoMengXinX/Music163bot-Go) / [Music163Api-Go](https://github.com/XiaoMengXinX/Music163Api-Go)（GPL-3.0）的 Eapi 协议实现，用 Rust 重写 AES-ECB 加密、请求签名、Cookie 构造。
- **SSE 实时进度推送** — `GET /api/admin/download/stream` 返回 `text/event-stream`，前端 `EventSource` 实时接收每行日志，取代原有 2 秒轮询。
- **下载后自动刷新播放队列** — 批量下载完成后自动发送 `AudioCommandType::ReloadQueue` 给音频引擎，新歌曲立即进入播放列表（不 Rescan 数据库）。
- **可配置并发** — `config.toml` `[ncm]` 段新增 `download_concurrency`（默认 1，最大 8）。
- **可配置 device_id** — `config.toml` `[ncm]` 段新增 `device_id`（留空则每次启动生成）。
- **移除 YouTube / Bilibili 降级** — 不再依赖 `yt-dlp`。

### 架构变更

- `radio-engine` 新增 `AudioCommandType::ReloadQueue`，支持运行时重载播放队列。
- `Cargo.toml` 新增依赖：`reqwest`、`aes`、`cipher`、`md-5`、`rand`、`hex`、`id3`、`tokio-stream`。
- 前端 `AdminDownload.vue` 从轮询改为 `EventSource` 监听 SSE。
- `build_release.sh` 移除 Python / `music_dl.py` 相关说明。

---

## v3.0.0 — 2026-05-05

### 重大架构变更：C++ 引擎被 Rust 重写替代

- **全新 Rust 音频引擎** (`radio-engine/` crate) — 原 C++ 音频引擎已移除，由 Rust 零拷贝环形缓冲区引擎替代。
  - 音频管道：ffmpeg → `RingBuffer` → TCP 客户端流
  - 播放列表扫描使用 ffmpeg 提取元数据（与旧版 C++ 引擎一致）
  - 内嵌为 `radio-backend` 内的本地 crate（非独立进程）
- **统一单二进制架构** — 不再有独立的 C++ 进程或通过 HTTP 进行进程间通信；引擎直接在 `radio-backend` 内运行。
  - 音频流端点 `/stream` 现在由与 API 相同的 HTTP 服务器（端口 2241）提供服务，而非 C++ 引擎的端口 2240。
  - 仅使用一个 PID 文件（`.server.pid`），管理单个进程。
- **移除的依赖**：不再需要 `crow_all.h`、`Makefile`、`libssl-dev`、`wget`、`libasio-dev`。构建仅需 `cargo` + `ffmpeg`（运行时依赖）。

### 认证方式：基于设备 → 取代 JWT/密码登录

- **设备认证**（`migrations/004_device_auth.sql`）— 通过 httpOnly `device_token` cookie 实现无密码认证。
  - 每台浏览器/设备在首次访问时自动注册设备用户。
  - 通过 `admin_setup_token`（在 `config.toml` 中配置）将设备用户提升为管理员。
  - 移除旧的基于密码的 `users` 表及其所有引用表。
  - 前端不再需要登录/注册弹窗、JWT token 存储或密码字段。
- **配置变更**：`[jwt]` 节 → 替换为 `[device]` 节（`cookie_max_age_days`、`admin_setup_token`）。
- **移除的前端组件**：`AuthModal.vue`（登录/注册 UI 不再需要）。

### 前端重构

- `store.ts`：将 `token` + `currentUser`（基于 JWT）替换为 `deviceUser`（基于 cookie）。
- `types.ts`：`User` → `DeviceUser`（`username` → `display_name`，新增 `device_token` 字段）。
- `router.ts`：导航守卫现在检查 `deviceUser.role` 而非 `currentUser.role`。
- `App.vue`：移除账户设置覆盖层；现在所有请求均自动完成设备认证。
- `api.ts`：移除 JWT token 处理；`loadDeviceUser()` 替代 `loadCurrentUser()`。

### 构建与启动简化

- `build_release.sh`：v3.0 — 编译对象仅为 Rust；不再有 C++ 编译、不再下载 `crow_all.h`、不再使用 `--no-crow` / `--skip-rust` 参数。
- `dist/start.sh` + `dist/stop.sh`：处理单个二进制文件；移除 `.rust-server.pid`。

### 数据库迁移

- 新增 `004_device_auth.sql`：创建 `device_users` 表及其所有引用表（`playlists`、`queue_items`、`play_history`、`admin_log`、`favorites`、`user_ncm`）。
- 旧的迁移 `002_seed_defaults.sql`（密码/管理员种子数据）将不再使用。

---

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
