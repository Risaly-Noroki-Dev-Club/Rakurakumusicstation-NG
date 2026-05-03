# Rakuraku Music Station NG

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![C++](https://img.shields.io/badge/C++-17-orange.svg)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

> **"The Long Last Goodbye"** — a farewell to the monolithic single-process era.  
> C++ handles the audio; Rust owns the web. Redis is the bridge.

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

## Quick Start

```bash
# 1. Get crow_all.h (one-time)
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 2. Build
./build_release.sh

# 3. Add audio files
cp /path/to/music/*.mp3 dist/media/

# 4. Start both services (C++ engine + Rust backend)
cd dist && ./start.sh

# Stop both services
./stop.sh
```

- Audio stream: `http://localhost:2240/stream`
- Health check: `http://localhost:2240/health`
- Web UI (Rust): `http://localhost:2241`

Stop: `./stop.sh`

## User Guide

Once the server is running, open your browser and go to `http://localhost:2241`.

### Getting Started
1. **Register** — Create an account on the login page
2. **Log in** — Sign in with your credentials to unlock all features

### Main Features
- **Now Playing** — See the current track, cover art, and lyrics on the home page
- **Song Library** — Browse and search all available songs
- **Request a Song** — Click "Add to Queue" to request your favorite tracks
- **Queue** — View the upcoming playlist and your queued songs
- **Favorites** — Heart songs to save them to your personal collection
- **Playlists** — Create and manage your own playlists
- **Player Controls** — Play, pause, volume, and seek in the built-in audio player

### Audio Stream
For external players (VLC, mpv, etc.), use the direct stream URL:
`http://localhost:2240/stream`

> **Tip:** The admin can upload new songs, manage users, and control playback via the Admin Panel.

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

### 架构

两个服务，一个 Redis：

| 服务 | 端口 | 职责 |
|------|------|------|
| **C++ 音频引擎** (`radioserver.cpp`) | 2240 | ffmpeg 管道 → 环形缓冲 → TCP 推流；向 Redis 发布 `PlaybackState`；订阅 `command` 频道接收控制指令 |
| **Rust 后端** (`radio-backend/`) | 2241 | REST API、WebSocket、多用户 JWT 认证、SQLite、队列/播放列表管理；通过 Redis 驱动 C++ 引擎 |

- Redis 对 C++ 引擎**可选** — 未连接时独立运行。
- 两者共享 `media/` 目录。

### 快速开始

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

# 停止服务
./stop.sh
```

- 音频流：`http://localhost:2240/stream`
- 健康检查：`http://localhost:2240/health`
- Web 界面（Rust）：`http://localhost:2241`

停止：`./stop.sh`

## 使用指南

服务启动后，在浏览器中打开 `http://localhost:2241` 即可访问。

### 快速上手
1. **注册账号** — 在登录页面点击「注册」创建账户
2. **登录** — 使用账号密码登录，解锁全部功能

### 主要功能
- **正在播放** — 首页显示当前曲目、封面图、歌词
- **曲库浏览** — 浏览和搜索所有可用歌曲
- **点歌** — 点击「加入队列」点播喜欢的歌曲
- **播放队列** — 查看即将播放的歌曲和你的点歌位置
- **收藏** — 点击心形图标收藏歌曲到个人收藏
- **歌单** — 创建和管理个人歌单
- **播放控制** — 使用内置播放器控制播放、暂停、音量、进度

### 外部播放器
如需使用外部播放器（VLC、mpv 等），直接流地址为：
`http://localhost:2240/stream`

> **提示：** 管理员可以通过管理面板上传歌曲、管理用户、控制播放。

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
