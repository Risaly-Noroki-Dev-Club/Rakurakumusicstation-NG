# Rakuraku Music Station NG — v3.1-beta

![License](https://img.shields.io/badge/license-MIT-blue.svg) ![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

> Rust 全栈：嵌入式音频引擎 + Web 后端 + Vue 3 前端。
> 一个自托管的网络电台，带现代 Web 界面、多设备支持、httpOnly Cookie 认证、WebSocket 实时同步和内嵌 Rust 音频引擎。

**v3.1-beta 重点：** 引擎-后端概念对齐、流地址自动推断、路径标准化、强类型状态机。

---

## 快速开始

### 从源码构建

```bash
# 安装依赖（Debian/Ubuntu）
apt install ffmpeg

# 构建（需 Rust toolchain）
./build_release.sh

# 放入音乐文件
cp /path/to/music/*.mp3 dist/media/

# 启动
cd dist && ./start.sh

# 浏览器打开 http://localhost:2241
```

### 停止服务

```bash
cd dist && ./stop.sh
```

### 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| Web 界面 + API + 音频流 + WebSocket | `2241` | 单端口，统一二进制文件 |

---

## 使用指南

### 首次启动

浏览器打开 `http://localhost:2241`，设备会自动获得一个 `device_token` Cookie。无密码，无需注册。

如需管理员权限：编辑 `dist/config.toml` 中的 `admin_setup_token`，然后点击导航栏 **🔑** 按钮输入该令牌即可升级为管理员。

### 页面导航

| 标签 | 功能 |
|------|------|
| **播放器** | 当前曲目、封面、进度条、同步歌词、音量控制 |
| **队列** | 前方排队歌曲 + 播放历史 |
| **曲库** | 搜索/浏览歌曲、点歌、下载、上传、歌单、网易云账号 |

### 点歌

曲库中搜索歌曲 → 点击 **📻 点歌** 即可加入电台队列。

### 下载歌曲

曲库列表中每首歌曲旁的 **⬇️** 按钮，点击即可下载到本地。

### 上传歌曲

曲库页面下方的 **📤 上传歌曲** 面板，支持 MP3/FLAC/WAV/OGG/M4A/AAC（最大 100 MB）。上传后自动加入曲库。

### 网易云账号

曲库页面下方的 **🎵 我的网易云账号** 面板，可配置个人网易云 Cookie 或手机号+密码。每个设备独立配置，互不影响。配置后可下载 VIP 歌曲。

### 播放器控制

- **播放/暂停** — 切换状态
- **音量** — 滑块调节
- **进度** — 点击/拖动进度条跳转
- **主题** — 🌓 按钮切换亮色/暗色/自动模式

### 外部播放器

VLC、mpv、ffplay 等可直接播放 `http://localhost:2241/stream`。

### PWA 支持

可在 Android/iOS 添加到主屏幕，离线缓存界面壳层。

---

## 管理员指南

管理员角色用户可看到 **管理** 标签页。

### 管理面板

| 子标签 | 功能 |
|--------|------|
| 👥 用户管理 | 查看设备列表、封禁/解封、提权/降权、操作日志 |
| 🎵 歌曲管理 | 查看/删除歌曲、重新扫描 media/、切歌 |
| 📤 上传 | 上传音频文件 |
| ⬇️ 下载 | 批量下载歌单（原生 Rust 网易云下载，SSE 实时进度） |
| 🎵 网易云 | 全局网易云账号配置 |
| ⚙️ 设置 | 电台名称、副标题、主题色 |
| 📊 统计 | 设备数、歌曲数、队列数、歌单数 |

### 提权设备

在 **用户管理** 中点击 **⭐ 提权** 可将普通设备提升为管理员，**⬇ 降权** 撤销。

备选方案（SQLite）：
```bash
sqlite3 dist/data/radio.db "UPDATE device_users SET role='admin' WHERE id=设备ID;"
```

### 重新扫描媒体库

直接向 `media/` 文件夹添加文件后，在 **歌曲管理** 中点击 **🔄 重新扫描**（需服务器安装 ffprobe）。

---

## 常见问题

| 问题 | 解决方法 |
|------|----------|
| 无法连接 Web 界面 | 确认服务已启动：`cd dist && ./start.sh`；查看日志 `tail -f dist/server.log` |
| 无声音/流不可用 | 确认 `media/` 中有音频文件；查看日志 `tail -f dist/server.log`；检查 `/api/station` 返回的 `stream_url` 是否正确 |
| 反向代理后流地址错误 | 确保反代传递 `Host`、`X-Forwarded-Host`、`X-Forwarded-Proto` 头；或手动设置 `stream_base` 为绝对 URL |
| 曲库无歌曲 | 在管理面板点击 **重新扫描** 或重启服务 |
| 无法获取管理员权限 | 确认 `dist/config.toml` 中 `admin_setup_token` 已设置 |
| 设置不生效 | 更改后需重启（`./stop.sh && ./start.sh`） |
| 封面不显示 | 确保音频文件含内嵌封面（ID3 标签）；缺失时显示默认音符图标 |
| 歌词不显示 | 仅支持同名 `.lrc` 文件放在同目录下；v3.1 后端已预解析歌词，前端无需额外配置 |

---

## 技术架构

```
media/  ──ffmpeg──▶  RingBuffer (radio-engine)  ──notify──▶  N 客户端
                      (零拷贝环形缓冲)           (async)     (HTTP /stream)
                             ▲
                     Player (fork ffmpeg → pipe → buffer)
                             │
             ┌───────────────┴──────────────┐
             │  Rust Backend (2241)          │
             │  嵌入 radio-engine crate      │
             │  Axum + SQLite + Device Auth  │
             │  WebSocket + 歌词 + 队列管理   │
             └──────────────────────────────┘
                             │
                     static/ (Vite-built Vue 3 SPA)
```

### 服务划分

| 组件 | 语言 | 说明 |
|------|------|------|
| 音频引擎 | Rust | `radio-engine/` crate，内嵌于后端，ffmpeg 解码 → 环形缓冲 → async 推流 |
| 业务后端 | Rust | `radio-backend/`，REST API、WebSocket、设备认证、SQLite、队列管理 |
| Web 前端 | TypeScript | Vue 3 SFC + Vite，构建产物在 `radio-backend/static/` |

### v3.1-beta 核心改进

| 改进项 | 说明 |
|--------|------|
| **stream_base = "auto"** | 自动检测反向代理（X-Forwarded-*）并构建正确的流地址，开箱即用 |
| **路径标准化** | engine 内统一使用相对路径存储，`resolve_media_path` 自动识别绝对/相对路径 |
| **概念对齐** | `PlaybackState.playlist_index`（原 `song_id`）、`PlaybackStatus` enum、`duration_ms` 统一 |
| **歌词预解析** | 后端解析 LRC 为结构化数组通过 WebSocket 推送，前端零解析开销 |
| **递归扫描** | engine `init_play_queue` 与 backend `rescan_songs` 均递归扫描子目录 |
| **线程安全** | `RingBuffer` `Condvar` → `tokio::sync::Notify`，消除 `/stream` 阻塞线程问题 |

---

## API 参考

### Rust 后端

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/api/station` | 无 | 电台信息 |
| `GET` | `/api/now-playing` | 无 | 当前曲目 |
| `GET` | `/api/songs?q=` | 无 | 曲库搜索 |
| `GET` | `/api/songs/:id` | 无 | 歌曲详情 |
| `GET` | `/api/songs/:id/cover` | 无 | 封面图片 |
| `GET` | `/api/songs/:id/download` | Device | 下载歌曲文件 |
| `POST` | `/api/songs/upload` | Device | 上传歌曲 |
| `GET` | `/api/queue` | 无 | 队列 |
| `GET` | `/api/queue/history` | 无 | 播放历史 |
| `POST` | `/api/queue` | Device | 点歌 |
| `GET` | `/api/user/me` | Device | 当前设备信息 |
| `POST` | `/api/user/display-name` | Device | 设置显示名称 |
| `POST` | `/api/user/promote` | 无 | 通过 admin_setup_token 提权 |
| `GET` `POST` `DELETE` | `/api/playlists` | Device | 歌单管理 |
| `GET` `POST` `DELETE` | `/api/favorites` | Device | 收藏管理 |
| `GET` `POST` | `/api/ncm` | Device | 个人网易云账号 |
| `POST` | `/api/ncm/test` | Device | 测试网易云登录 |
| `*` | `/api/admin/*` | Admin | 管理端点 |
| `WS` | `/ws` | Device (query) | WebSocket 实时推送 |

### 音频流

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/stream` | 音频流 (`Content-Type: audio/mpeg`) |

---

## 配置文件 (`config.toml`)

```toml
[server]          # host、port（默认 2241）
[database]        # SQLite URL
[audio_engine]    # media_path、stream_base（auto / 相对路径 / 绝对 URL）
[device]          # cookie_max_age_days、admin_setup_token
[queue]           # max_size、rate_limit
[station]         # name、subtitle、主题色
[logging]         # level
```

### `stream_base` 三种模式

| 值 | 场景 |
|----|------|
| `"auto"` | **推荐。** 自动根据 `Host` / `X-Forwarded-*` 请求头推断流地址，适用于大多数部署（含反向代理）。 |
| `"/stream"` | 相对路径。前端用 `window.location.origin` 解析，适用于简单的内网直连。 |
| `"http://cdn.example.com/stream"` | 强制绝对 URL。适用于 CDN 或独立流服务器。 |

---

## 构建手册

```bash
# 依赖：ffmpeg、Rust toolchain

# 一键构建
./build_release.sh

# 前端（如修改了源码）
cd radio-backend/frontend && npm install && npm run build

# 仅 Rust
cd radio-backend && cargo build --release
```

支持格式：MP3、WAV、FLAC、OGG、M4A、AAC

---

## License

MIT

本项目包含以下第三方代码：

- **网易云下载模块** (`radio-backend/src/services/ncm/`) 的 Eapi 加密逻辑与 API 调用参考自 [Music163bot-Go](https://github.com/XiaoMengXinX/Music163bot-Go)（GPL-3.0 许可证）。相关代码在 Rust 中进行了重写和适配。

## Credits

- 知夏 (Zhixia) — 项目协作者
- [FFmpeg](https://ffmpeg.org/) — 音频解码
- [Axum](https://github.com/tokio-rs/axum) — Rust HTTP 框架
- [Vue 3](https://vuejs.org/) — 前端框架
- [Vite](https://vitejs.dev/) — 前端构建工具
- [SQLx](https://github.com/launchbadge/sqlx) — Rust SQL 工具集
- [Music163bot-Go](https://github.com/XiaoMengXinX/Music163bot-Go) / [Music163Api-Go](https://github.com/XiaoMengXinX/Music163Api-Go) — 网易云音乐 API 实现参考（GPL-3.0）

灵感来源：《孤独摇滚！》— 伊地知虹夏
