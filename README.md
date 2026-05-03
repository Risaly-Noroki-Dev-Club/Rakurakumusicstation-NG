# Rakuraku Music Station NG

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![C++](https://img.shields.io/badge/C++-17-orange.svg)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

> **"The Long Last Goodbye"** — a farewell to the monolithic single-process era.  
> C++ handles the audio; Rust owns the web. Redis is the bridge.

A self-hosted internet radio station with a modern web UI, multi-user support, JWT authentication, real-time playback sync via WebSocket, and a high-performance C++ audio engine.

---

## For Users — Getting Started

### System Requirements

- **Server**: Linux (x86_64 recommended), ffmpeg, optionally Redis
- **Client**: Any modern browser (Chrome, Firefox, Safari, Edge) — desktop or mobile

### Installation (Pre-built)

1. Download the latest release from the [Releases](https://github.com/Risaly-Noroki-Dev-Club/Rakurakumusicstation-NG/releases) page
2. Extract the archive into a directory
3. Place your music files (MP3, FLAC, WAV, OGG, M4A, AAC) into `dist/media/`
4. Run `cd dist && ./start.sh`
5. Open `http://localhost:2241` in your browser

### Installation (Build from Source)

```bash
# 1. Download crow_all.h (one-time)
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 2. Build
./build_release.sh

# 3. Add audio files
cp /path/to/music/*.mp3 dist/media/

# 4. Start both services (C++ engine + Rust backend)
cd dist && ./start.sh

# 5. Open in browser
# → http://localhost:2241
```

### Stopping the Server

```bash
cd dist && ./stop.sh
```

### Service URLs

| Service | URL |
|---------|-----|
| Web UI | `http://localhost:2241` |
| Audio Stream (for external players) | `http://localhost:2240/stream` |
| Health Check | `http://localhost:2240/health` |

---

## User Guide

Once the server is running, open your browser and navigate to `http://localhost:2241`.

### First Visit — Registration & Login

1. Click **登录 / Login** in the top-right corner
2. Switch to **注册 / Register** tab
3. Choose a username and password — no email required
4. Click **注册 / Register**
5. Log in with your new credentials
6. Your login persists across browser sessions (JWT stored in localStorage)

> **Admin account**: If you need admin access, the first registered user cannot be admin by default. See [Admin Setup](#admin-setup) below.

### Home Page — Now Playing

The home page shows:

- **Cover Art** — Album artwork of the current song (falls back to a music-note icon if unavailable)
- **Song Info** — Title, artist, and album
- **Progress Bar** — Click or drag to seek within the track
- **Time Display** — Current position / total duration
- **Volume Slider** — Adjust playback volume
- **Synced Lyrics** — LRC-formatted lyrics scroll in sync with playback (if available)

The page auto-refreshes in real time via WebSocket — no need to reload.

### Song Library — Browsing & Searching

Click the **曲库 / Library** tab to browse all available songs.

- **Search**: Type in the search box to filter by title or artist
- **Browse**: Scroll through the song list
- **Add to Queue**: Click the **＋** button next to any song to request it
- **Add to Playlist**: Click the playlist icon to add a song to one of your playlists

### Queue — Requesting Songs

Click the **队列 / Queue** tab to see the upcoming playlist.

- **Request a Song**: From the Library tab, click **＋** on any song, or click **加入队列 / Add to Queue**
- **View Queue**: See all queued songs in order, with requester names
- **History**: Switch to the **历史 / History** sub-tab to see recently played tracks
- **Remove** (admin only): Admins can remove items from the queue

### Favorites — Heart Songs

- Click the **♥** icon on any song in the Library or Now Playing view
- Favorited songs appear in your personal **收藏 / Favorites** list
- Click **♥** again to unfavorite

### Playlists — Curate Your Own

Click the **歌单 / Playlists** sub-tab under Library.

- **Create**: Click **新建歌单 / New Playlist**, give it a name
- **Add Songs**: From the library, click the playlist icon on any song and choose a playlist
- **View**: Click a playlist name to see its contents
- **Delete**: Remove playlists or individual songs from playlists

### Player Controls

- **Play/Pause** — Toggle playback
- **Volume** — Slider control (0–100%)
- **Seek** — Click or drag the progress bar
- **Theme** — Click the 🌓 icon to cycle through Light / Dark / Auto modes

### Dark Mode

Three modes available via the 🌓 button in the top bar:

| Mode | Behavior |
|------|----------|
| **Auto** (default) | Follows your OS/browser color scheme |
| **Light** | Always light theme |
| **Dark** | Always dark theme |

Your preference is saved in your browser.

### Using External Players

You can listen with any audio player that supports HTTP streams:

- **VLC**: `Media → Open Network Stream` → enter `http://localhost:2240/stream`
- **mpv**: `mpv http://localhost:2240/stream`
- **ffplay**: `ffplay http://localhost:2240/stream`
- **Any browser**: Navigate directly to `http://localhost:2240/stream`

> External players receive the live broadcast — you won't see lyrics or track info. Use the Web UI for the full experience.

### Mobile / PWA

The web UI is a Progressive Web App (PWA):

- **Install**: On Android Chrome, tap **⋮ → Add to Home Screen**. On iOS Safari, tap **Share → Add to Home Screen**.
- **Offline Shell**: The app caches its shell for faster loading, though playback requires a network connection to the server.
- **Responsive**: The UI adapts to phone, tablet, and desktop screen sizes.

---

## Admin Guide — For Station Owners

Access the Admin Panel by clicking the **管理 / Admin** tab (visible only to users with the `admin` role).

### Admin Setup

By default, no user has admin privileges. To grant admin access:

1. Register a normal user account first
2. Use SQLite directly to promote the user:

```bash
cd dist
sqlite3 data/radio.db "UPDATE users SET role='admin' WHERE username='your-username';"
```

3. Log out and log back in — the **管理 / Admin** tab will appear

### Admin Sub-tabs

| Tab | Functions |
|-----|-----------|
| **用户管理 / Users** | View all users; ban/unban users; view operation logs |
| **歌曲管理 / Songs** | View all songs; delete songs; re-scan `media/` directory for new files; skip current track |
| **上传 / Upload** | Upload audio files (up to 100 MB) directly through the browser |
| **下载 / Download** | Batch download songs via `music_dl.py` with real-time progress logs |
| **网易云 / NCM** | Log in with NetEase Cloud Music cookie or phone number; test login status |
| **设置 / Settings** | Change station name, subtitle, theme colors; set admin password |
| **统计 / Stats** | View counts: total users, songs, queue items, playlists |

> **Note**: Settings changes (station name, theme colors, password) take effect after restarting the server (`./stop.sh && ./start.sh`).

### Uploading Songs

1. Go to **管理 / Admin → 上传 / Upload**
2. Click to select an audio file (MP3, FLAC, WAV, OGG, M4A, AAC — max 100 MB)
3. Click **上传 / Upload**
4. The file is saved to `media/` and automatically added to the song library
5. Cover art embedded in the audio file's metadata is extracted automatically

### Re-scanning Media

If you added files directly to the `media/` folder (via SCP, SFTP, etc.):

1. Go to **管理 / Admin → 歌曲管理 / Songs**
2. Click **重新扫描 / Rescan**
3. The server scans `media/` for new files and adds them to the library

> Requires `ffprobe` (part of ffmpeg) to be installed on the server for duration extraction.

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| **Can't connect to Web UI** | Ensure both services are running: `cd dist && ./start.sh`. Check logs: `tail -f dist/rust-server.log` |
| **No audio / stream not working** | Verify `media/` contains audio files. Check C++ engine logs: `tail -f dist/server.log` |
| **No songs in library** | Click **重新扫描 / Rescan** in Admin → Songs, or restart the server |
| **Can't log in** | The default admin account has a placeholder password hash. Register a new account instead |
| **Queue doesn't advance** | The C++ engine must be connected to Redis for Rust to send commands. Or restart the server |
| **Settings don't apply** | Settings changes require a server restart (`./stop.sh && ./start.sh`) |
| **Cover art not showing** | Ensure audio files have embedded cover art (ID3 tags). Missing covers show a default music icon |
| **Lyrics not showing** | Only LRC files (`.lrc` next to the audio file with the same name) are supported |

---

## Architecture

```
media/  ──FFmpeg──▶  BroadcastBuffer  ──epoll──▶  N listeners
                     (lock-free ring)   (fan-out)  (HTTP /stream)
                            ▲
                    AudioPlayer (fork ffmpeg → pipe → buffer)
                            │ ╲
                Redis ──────┘  └────── Redis
               pub/sub               pub/sub
                  │                     │
            playback_state          command
                  │                     │
                  ▼                     ▼
         Rust Backend (port 2241) ◀──────────┘
         Axum + SQLite + JWT + WebSocket
                  │
         static/ (Web UI)
```

Two services, one Redis:

| Service | Port | Role |
|---------|------|------|
| **C++ Audio Engine** (`radioserver.cpp`) | 2240 | ffmpeg pipeline → ring buffer → TCP stream; publishes `PlaybackState` to Redis; subscribes to `command` channel |
| **Rust Backend** (`radio-backend/`) | 2241 | REST API, WebSocket, multi-user JWT auth, SQLite, queue/playlist management; drives C++ engine via Redis |

- Redis is **optional** for the C++ engine — it runs standalone if unavailable.
- Both services share the `media/` directory for audio files.

---

## Build (Manual)

```bash
# Dependencies: g++, ffmpeg, libssl-dev, libasio-dev, python3

# Build hiredis (one-time)
(cd third_party/hiredis && \
  gcc -c -O2 -I. hiredis.c alloc.c async.c net.c read.c sds.c sockcompat.c dict.c && \
  ar rcs libhiredis.a *.o && rm -f *.o)

# Debug build
g++ radioserver.cpp metadata.cpp -o radioserver -std=c++17 -g -O0 \
  -lpthread -lssl -lcrypto -I. -Ithird_party/hiredis \
  third_party/hiredis/libhiredis.a

# Release build
g++ radioserver.cpp metadata.cpp -o dist/radioserver \
  -std=c++17 -O3 -flto -march=native -lpthread -lssl -lcrypto \
  -I. -Ithird_party/hiredis -w third_party/hiredis/libhiredis.a

# Rust backend
cd radio-backend && cargo build --release
```

## C++ Audio Engine — API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/stream` | Audio stream (`Content-Type: audio/mpeg`); takes over TCP socket |
| `GET` | `/health` | `{"status":"ok","service":"rakuraku-audio-engine","clients":N}` |

No auth, no admin panel — all management goes through the Rust backend.

## Rust Backend — API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/station` | No | Station info + theme colors |
| `GET` | `/api/now-playing` | No | Current track + `lyrics_line` / `lyrics_text` |
| `GET` | `/api/songs?q=` | No | Song library search |
| `GET` | `/api/songs/{id}/cover` | No | Cover art (JPEG/PNG) |
| `GET` | `/api/queue` | No | Shared queue |
| `GET` | `/api/queue/history` | No | Play history |
| `POST` | `/api/queue` | JWT | Add song to queue |
| `DELETE` | `/api/queue/{id}` | Admin | Remove queue item |
| `POST` | `/api/queue/skip` | Admin | Skip track (→ Redis `command`) |
| `POST` | `/api/auth/register` | No | Register |
| `POST` | `/api/auth/login` | No | Login → JWT |
| `GET` | `/api/auth/me` | JWT | Current user info |
| `GET` | `/api/favorites` | JWT | User favorites |
| `POST` `DELETE` | `/api/favorites/{id}` | JWT | Add/remove favorite |
| `GET` `POST` `DELETE` | `/api/playlists` | JWT | Playlist CRUD |
| `GET` `POST` `DELETE` | `/api/admin/*` | Admin | User management, stats, song upload/delete, settings, download, NCM |
| `WS` | `/ws` | No | WebSocket: real-time `playback_state` + `queue_update` |

## Redis Channels

| Channel | Direction | Payload |
|---------|-----------|---------|
| `playback_state` | C++ → Rust | `PlaybackState` JSON (song_id, file_path, position_ms, duration_ms, status, total_bytes_sent, bitrate_kbps, track_start_timestamp_ms) |
| `command` | Rust → C++ | `AudioCommand` JSON (`type`: skip/prev/play/stop, `file_path`, `song_id`) |
| `queue_event` | Rust → C++ | `QueueEvent` JSON (queue changes — for future use) |

## Configuration

### C++ Engine

No configuration file needed. Scans `media/` on startup. Playlist order persists in `playlist_order.json`.

### Rust Backend — `config.toml`

```toml
[server]          # host, port (default 2241)
[database]        # SQLite URL
[redis]           # URL, channel names
[audio_engine]    # C++ engine base_url, media_path
[jwt]             # secret, expiry_hours
[queue]           # max_size, rate_limit
[station]         # name, subtitle, theme colors
[logging]         # level
```

## Supported Formats

MP3, WAV, FLAC, OGG, M4A, AAC

---

## 中文 / Chinese

### 面向用户 — 快速开始

#### 系统需求

- **服务器**：Linux（推荐 x86_64），ffmpeg，可选 Redis
- **客户端**：任何现代浏览器（Chrome、Firefox、Safari、Edge）— 桌面或移动端

#### 安装（预编译版本）

1. 从 [Releases](https://github.com/Risaly-Noroki-Dev-Club/Rakurakumusicstation-NG/releases) 下载最新版本
2. 解压到任意目录
3. 将音乐文件（MP3、FLAC、WAV、OGG、M4A、AAC）放入 `dist/media/`
4. 运行 `cd dist && ./start.sh`
5. 浏览器打开 `http://localhost:2241`

#### 安装（从源码构建）

```bash
# 1. 下载 crow_all.h（一次性）
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 2. 构建
./build_release.sh

# 3. 放入音频文件
cp /path/to/music/*.mp3 dist/media/

# 4. 启动服务（C++ 引擎 + Rust 后端）
cd dist && ./start.sh

# 5. 浏览器打开
# → http://localhost:2241
```

#### 停止服务

```bash
cd dist && ./stop.sh
```

#### 服务地址

| 服务 | 地址 |
|------|------|
| Web 界面 | `http://localhost:2241` |
| 音频流（外部播放器用） | `http://localhost:2240/stream` |
| 健康检查 | `http://localhost:2240/health` |

---

### 使用指南

服务启动后，在浏览器中打开 `http://localhost:2241` 即可访问。

#### 首次使用 — 注册与登录

1. 点击右上角 **登录 / Login**
2. 切换到 **注册 / Register** 标签
3. 设置用户名和密码（无需邮箱）
4. 点击 **注册 / Register**
5. 使用新账号登录
6. 登录状态自动保存（JWT 存在 localStorage 中），关闭浏览器后仍有效

> **管理员账号**：默认注册的用户没有管理员权限。详见下方[管理员设置](#管理员设置)。

#### 首页 — 正在播放

首页展示：

- **封面图** — 当前歌曲的专辑封面（无封面时显示默认音符图标）
- **歌曲信息** — 标题、艺人、专辑名
- **进度条** — 点击或拖动可跳转播放进度
- **时间显示** — 当前播放位置 / 总时长
- **音量滑块** — 调节播放音量
- **同步歌词** — 支持 LRC 格式歌词，随播放进度自动滚动

页面通过 WebSocket 实时更新，无需手动刷新。

#### 曲库 — 浏览与搜索

点击 **曲库 / Library** 标签浏览所有可用歌曲。

- **搜索**：在搜索框中输入关键词，按标题或艺人名筛选
- **浏览**：滚动查看歌曲列表
- **点歌**：点击歌曲旁的 **＋** 按钮加入播放队列
- **加入歌单**：点击歌单图标将歌曲添加到你的个人歌单

#### 队列 — 点歌系统

点击 **队列 / Queue** 标签查看即将播放的歌曲。

- **点歌**：在曲库中点击 **＋** 或 **加入队列 / Add to Queue**
- **查看队列**：按顺序显示排队歌曲，附点歌人昵称
- **播放历史**：切换到 **历史 / History** 子标签查看已播放曲目
- **移除**（仅管理员）：可移除队列中的歌曲

#### 收藏 — 红心歌曲

- 点击曲库或播放页面的 **♥** 图标收藏歌曲
- 已收藏歌曲在 **收藏 / Favorites** 列表中显示
- 再次点击 **♥** 取消收藏

#### 歌单 — 创建个人歌单

在曲库下点击 **歌单 / Playlists** 子标签。

- **新建**：点击 **新建歌单 / New Playlist**，输入名称
- **添加歌曲**：在曲库中点击歌曲旁的歌单图标，选择目标歌单
- **查看**：点击歌单名称查看内容
- **删除**：可删除歌单或从歌单中移除单曲

#### 播放器控制

- **播放/暂停** — 切换播放状态
- **音量** — 滑块调节（0–100%）
- **进度跳转** — 点击或拖动进度条
- **主题切换** — 点击 🌓 图标在亮色/暗色/自动三种模式间切换

#### 暗色模式

通过顶部 🌓 按钮切换三种模式：

| 模式 | 行为 |
|------|------|
| **自动**（默认） | 跟随操作系统/浏览器的颜色方案 |
| **亮色** | 始终使用亮色主题 |
| **暗色** | 始终使用暗色主题 |

偏好设置保存在浏览器中。

#### 使用外部播放器

支持任何能播放 HTTP 流的播放器：

- **VLC**：`媒体 → 打开网络串流` → 输入 `http://localhost:2240/stream`
- **mpv**：`mpv http://localhost:2240/stream`
- **ffplay**：`ffplay http://localhost:2240/stream`
- **浏览器**：直接访问 `http://localhost:2240/stream`

> 外部播放器接收的是直播音频流，无法显示歌词和曲目信息。建议使用 Web 界面获得完整体验。

#### 移动端 / PWA

Web 界面支持渐进式 Web 应用（PWA）：

- **安装**：Android Chrome 点击 **⋮ → 添加到主屏幕**。iOS Safari 点击 **分享 → 添加到主屏幕**。
- **离线外壳**：界面壳层可缓存，但播放仍需网络连接服务器。
- **响应式**：界面自适应手机、平板、桌面屏幕尺寸。

---

### 管理员指南 — 电台运营者

管理员权限用户可看到 **管理 / Admin** 标签页。

#### 管理员设置

默认没有用户拥有管理员权限。授予方法：

1. 先注册一个普通账号
2. 使用 SQLite 直接提升权限：

```bash
cd dist
sqlite3 data/radio.db "UPDATE users SET role='admin' WHERE username='你的用户名';"
```

3. 退出登录后重新登录 — **管理 / Admin** 标签即会出现

#### 管理子标签

| 标签 | 功能 |
|------|------|
| **用户管理 / Users** | 查看所有用户；封禁/解封用户；查看操作日志 |
| **歌曲管理 / Songs** | 查看所有歌曲；删除歌曲；重新扫描 media/ 目录；切歌 |
| **上传 / Upload** | 通过浏览器直接上传音频文件（最大 100 MB） |
| **下载 / Download** | 通过 `music_dl.py` 批量下载歌曲，实时显示日志 |
| **网易云 / NCM** | 网易云音乐 Cookie 或手机号登录 + 登录状态测试 |
| **设置 / Settings** | 修改电台名称、副标题、主题色；设置管理员密码 |
| **统计 / Stats** | 查看用户数、歌曲数、队列数、歌单数统计 |

> **注意**：设置更改（电台名、主题色、密码）需要重启服务器生效（`./stop.sh && ./start.sh`）。

#### 上传歌曲

1. 进入 **管理 / Admin → 上传 / Upload**
2. 选择音频文件（MP3、FLAC、WAV、OGG、M4A、AAC — 最大 100 MB）
3. 点击 **上传 / Upload**
4. 文件保存至 `media/`，自动加入曲库
5. 音频文件内嵌的封面图将被自动提取

#### 重新扫描媒体库

如果通过 SCP、SFTP 等方式直接向 `media/` 添加了文件：

1. 进入 **管理 / Admin → 歌曲管理 / Songs**
2. 点击 **重新扫描 / Rescan**
3. 服务器扫描 `media/` 目录中的新文件并加入曲库

> 需要服务器安装 `ffprobe`（ffmpeg 组件）以提取音频时长信息。

---

### 常见问题

| 问题 | 解决方法 |
|------|----------|
| **无法连接 Web 界面** | 确保两个服务已启动：`cd dist && ./start.sh`。查看日志：`tail -f dist/rust-server.log` |
| **没有声音/流不可用** | 确认 `media/` 中有音频文件。查看 C++ 引擎日志：`tail -f dist/server.log` |
| **曲库中没有歌曲** | 在管理面板 → 歌曲管理中点击 **重新扫描 / Rescan**，或重启服务 |
| **无法登录** | 默认管理员密码哈希为占位符。请注册新账号 |
| **队列不自动前进** | C++ 引擎需连接 Redis 才能接收 Rust 发送的指令。或重启服务 |
| **设置不生效** | 设置更改需要重启服务（`./stop.sh && ./start.sh`） |
| **封面图不显示** | 确保音频文件包含内嵌封面（ID3 标签）。缺失封面时显示默认音符图标 |
| **歌词不显示** | 仅支持 LRC 格式歌词文件（`.lrc`，与音频文件同名，放在同目录下） |

---

### 架构

两个服务，一个 Redis：

| 服务 | 端口 | 职责 |
|------|------|------|
| **C++ 音频引擎** (`radioserver.cpp`) | 2240 | ffmpeg 管道 → 环形缓冲 → TCP 推流；向 Redis 发布 `PlaybackState`；订阅 `command` 频道接收控制指令 |
| **Rust 后端** (`radio-backend/`) | 2241 | REST API、WebSocket、多用户 JWT 认证、SQLite、队列/播放列表管理；通过 Redis 驱动 C++ 引擎 |

- Redis 对 C++ 引擎**可选** — 未连接时独立运行。
- 两者共享 `media/` 目录。

### 手动构建

```bash
# 依赖：g++、ffmpeg、libssl-dev、libasio-dev、python3

# 编译 hiredis（一次性）
(cd third_party/hiredis && \
  gcc -c -O2 -I. hiredis.c alloc.c async.c net.c read.c sds.c sockcompat.c dict.c && \
  ar rcs libhiredis.a *.o && rm -f *.o)

# Debug 构建
g++ radioserver.cpp metadata.cpp -o radioserver -std=c++17 -g -O0 \
  -lpthread -lssl -lcrypto -I. -Ithird_party/hiredis \
  third_party/hiredis/libhiredis.a

# Release 构建
g++ radioserver.cpp metadata.cpp -o dist/radioserver \
  -std=c++17 -O3 -flto -march=native -lpthread -lssl -lcrypto \
  -I. -Ithird_party/hiredis -w third_party/hiredis/libhiredis.a

# Rust 后端
cd radio-backend && cargo build --release
```

### C++ 音频引擎 — API

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/stream` | 音频流（`Content-Type: audio/mpeg`），接管 TCP 套接字 |
| `GET` | `/health` | `{"status":"ok","service":"rakuraku-audio-engine","clients":N}` |

无认证、无管理面板 — 所有管理操作通过 Rust 后端完成。

### Rust 后端 — API

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/api/station` | 无 | 电台信息 + 主题色 |
| `GET` | `/api/now-playing` | 无 | 当前曲目 + `lyrics_line` / `lyrics_text` |
| `GET` | `/api/songs?q=` | 无 | 曲库搜索 |
| `GET` | `/api/songs/{id}/cover` | 无 | 封面图片 |
| `GET` | `/api/queue` | 无 | 队列 |
| `GET` | `/api/queue/history` | 无 | 播放历史 |
| `POST` | `/api/queue` | JWT | 点歌 |
| `DELETE` | `/api/queue/{id}` | Admin | 移除队列项 |
| `POST` | `/api/queue/skip` | Admin | 切歌（→ Redis `command`） |
| `POST` | `/api/auth/register` | 无 | 注册 |
| `POST` | `/api/auth/login` | 无 | 登录 → JWT |
| `GET` | `/api/auth/me` | JWT | 当前用户 |
| `GET` | `/api/favorites` | JWT | 收藏 |
| `POST` `DELETE` | `/api/favorites/{id}` | JWT | 添加/取消收藏 |
| `GET` `POST` `DELETE` | `/api/playlists` | JWT | 歌单 CRUD |
| `GET` `POST` `DELETE` | `/api/admin/*` | Admin | 用户管理、统计、歌曲上传/删除、设置、下载、NCM |
| `WS` | `/ws` | 无 | WebSocket：实时推送 `playback_state` + `queue_update` |

### Redis 频道

| 频道 | 方向 | 负载 |
|------|------|------|
| `playback_state` | C++ → Rust | `PlaybackState` JSON（song_id、file_path、position_ms、duration_ms、status、total_bytes_sent、bitrate_kbps、track_start_timestamp_ms） |
| `command` | Rust → C++ | `AudioCommand` JSON（`type`：skip/prev/play/stop、`file_path`、`song_id`） |
| `queue_event` | Rust → C++ | `QueueEvent` JSON（队列变更 — 预留） |

### 配置

**C++ 引擎** — 无需配置文件。启动时扫描 `media/` 目录。播放顺序保存在 `playlist_order.json`。

**Rust 后端** — `config.toml`

```toml
[server]          # host, port（默认 2241）
[database]        # SQLite URL
[redis]           # URL、频道名
[audio_engine]    # C++ 引擎 base_url、media_path
[jwt]             # secret、expiry_hours
[queue]           # max_size、rate_limit
[station]         # name、subtitle、主题色
[logging]         # level
```

### 支持格式

MP3、WAV、FLAC、OGG、M4A、AAC

---

## License

MIT

## Credits

- **知夏 (Zhixia)** — 项目协作者
- [Crow](https://github.com/CrowCpp/Crow) — C++ HTTP 框架
- [hiredis](https://github.com/redis/hiredis) — C Redis 客户端
- [FFmpeg](https://ffmpeg.org/) — 音频解码
- [Axum](https://github.com/tokio-rs/axum) — Rust HTTP 框架
- [Vue 3](https://vuejs.org/) — 前端框架 (CDN, 无构建)
- [SQLx](https://github.com/launchbadge/sqlx) — Rust SQL 工具集
- 灵感来源：《孤独摇滚！》— 伊地知虹夏
