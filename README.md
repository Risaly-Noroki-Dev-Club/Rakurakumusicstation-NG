# Rakuraku Music Station NG

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey.svg)
![C++](https://img.shields.io/badge/C++-17-orange.svg)

**Languages / 语言**: [English](#english) · [中文](#中文)

> **"NG" = Nijika Generation** — a tribute to Nijika Ijichi from *Bocchi the Rock!*, reflecting the spirit of sharing music joyfully.

---

## English

### Overview

Rakuraku Music Station is a C++ streaming radio server. One FFmpeg decoder feeds a lock-free ring buffer; a Linux `epoll` loop fans the audio bytes out to every connected listener. The Crow-based web panel handles playlist management, uploads, playback control, and settings.

### Features

- **Radio-style broadcast** — single decoder, shared ring buffer, all listeners hear the same stream in sync
- **Low-latency `epoll` I/O** — many concurrent listeners on one thread
- **Format support** — MP3, WAV, FLAC, OGG, M4A, AAC via FFmpeg
- **Hot-reload playlist** — uploads appear immediately; no restart needed
- **Metadata / cover art / lyrics** — per-track API endpoints
- **Session auth** — cookie-based admin sessions; optional guest skip
- **Themeable UI** — station name, colors, and subtitle pulled from `settings.json`
- **PWA** — service worker caching, theme color, mobile-optimized layout
- **Batch music download** — NetEase Cloud Music (pyncm) and YouTube (yt-dlp) via `music_dl.py`
- **Web settings panel** — edit configuration without touching `settings.json` manually
- **One-shot build** — `build_release.sh` installs deps, generates embedded templates, compiles, and produces `dist/`

### Requirements

- Linux (Arch, Debian, or Ubuntu — the build script auto-detects)
- GCC/G++ 7+ with C++17
- `ffmpeg`, `openssl`, `libasio`, `wget`, `curl`, `python3`
- Python 3.8+ with `pip install -r requirements.txt` (for the download feature)

### Quick Start

```bash
# 1. Build
./build_release.sh

# 2. Add audio files
cp /path/to/music/*.mp3 dist/media/

# 3. Run
cd dist && ./start.sh
```

Open <http://localhost:2240> in a browser.  
Audio stream: <http://localhost:2240/stream>

Stop with `./stop.sh`.

### Manual Build (Development)

```bash
# Debian/Ubuntu deps
sudo apt-get install build-essential ffmpeg libssl-dev libasio-dev wget curl locales

# Crow header (latest release)
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# Debug build
g++ radioserver.cpp metadata.cpp -o radioserver \
    -std=c++17 -g -O0 -lpthread -lssl -lcrypto -I.
```

> **`crow_all.h` is a prerequisite** — the build script does not download it. Download it once before building (see the `wget` command above). Delete the file to pull a newer version.

### Configuration — `settings.json`

```json
{
    "station_name": "Rakuraku Music Station",
    "subtitle": "Your tagline here",
    "primary_color": "#764ba2",
    "secondary_color": "#667eea",
    "bg_color": "#f4f4f9",
    "admin_password": "change_me",
    "allow_guest_skip": false,
    "ncm_phone": "",
    "ncm_password": "",
    "ncm_cookie": ""
}
```

| Key | Default | Notes |
|-----|---------|-------|
| `admin_password` | `admin123` | **Change before exposing publicly** |
| `allow_guest_skip` | `false` | Lets unauthenticated users call `/api/next` and `/api/prev` |
| `ncm_*` | empty | NetEase Cloud Music credentials; cookie takes precedence over phone+password |

### HTTP API

**Public**

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Listener page (admin panel if logged in) |
| GET | `/api/playlist` | Playlist + per-track metadata |
| GET | `/api/stats` | Current listener count |
| GET | `/api/metadata/<idx>` | Full metadata for track `idx` |
| GET | `/api/cover/<idx>` | Embedded cover art |
| GET | `/api/lyrics/<idx>` | Lyrics text |

**Admin** (session cookie required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/admin/login` | `{"password":"..."}` → sets `session_id` cookie |
| POST | `/admin/logout` | Destroy session |
| POST | `/upload` | Multipart upload (≤ 50 MB) |
| POST | `/api/next` | Skip forward |
| POST | `/api/prev` | Skip backward |
| POST | `/api/play/<idx>` | Jump to track `idx` |
| POST | `/api/delete/<idx>` | Remove track `idx` |
| POST | `/admin/download` | Batch download from playlist file |
| GET | `/admin/download/status` | Poll download progress |
| GET | `/admin/settings/get` | Read all settings |
| POST | `/admin/settings/save` | Save settings |
| GET | `/admin/settings/ncm` | Read NetEase config status |
| POST | `/admin/settings/ncm` | Save NetEase credentials |
| POST | `/admin/settings/ncm/test` | Test NetEase login |

### Architecture

| Component | Role |
|-----------|------|
| `RadioServer` | Top-level coordinator |
| `BroadcastBuffer` | Power-of-two ring buffer — one producer, many consumers |
| `StreamServer` | `epoll` loop fanning audio bytes to listeners |
| `AudioPlayer` | Spawns FFmpeg with `-re`, pipes decoded audio into the buffer |
| `WebServer` | Crow app serving UI and REST APIs |
| `SessionManager` | In-memory session table with 24-hour sliding expiry |
| `MetadataManager` | Extracts title/artist/duration via filename and `ffmpeg -i` |

### Project Layout

```
├── radioserver.cpp        # Main server (all core classes)
├── metadata.{hpp,cpp}     # Audio metadata extraction
├── sessionmanager.hpp     # Session store
├── authmiddleware.hpp     # Auth middleware (referenced in docs, not yet on disk)
├── build_release.sh       # One-shot build script
├── music_dl.py            # NetEase/YouTube download script
├── requirements.txt       # Python dependencies
├── manifest.json          # PWA web manifest
├── sw.js                  # Service worker
├── *.html                 # HTML templates
└── dist/                  # Build output (preserved across rebuilds)
    ├── radioserver
    ├── start.sh / stop.sh
    ├── media/             # Audio files
    ├── settings.json      # Runtime config
    └── templates/         # Optional custom templates (override embedded)
```

### Troubleshooting

| Symptom | Fix |
|---------|-----|
| Garbled non-ASCII filenames | Run `sudo locale-gen zh_CN.UTF-8 && sudo update-locale LANG=zh_CN.UTF-8`, or start via `./start.sh` |
| Port already in use | `ss -ltnp \| grep 2240`; edit `Config::WEB_PORT` in `radioserver.cpp` |
| `revents=16` pipe error | Locale issue — see above |
| No audio | Confirm `ffmpeg` is on `PATH`; check the file extension is in the supported list |
| Download fails | Check Python deps (`pip install -r requirements.txt`) and NetEase credentials |

Logs: `dist/server.log`

### Security

- Change `admin_password` before exposing the server publicly.
- Sessions use `HttpOnly`, `SameSite=Lax` cookies — put a TLS-terminating reverse proxy in front for public deployments.
- Uploads cap at 50 MB and reject unsupported extensions.

### License

MIT — see [LICENSE](LICENSE).

### Credits

- **知夏 (Zhixia)** — project collaborator
- **[pyncm](https://github.com/HexiNail/pyncm)** — NetEase Cloud Music Python API
- **[Crow](https://github.com/CrowCpp/Crow)** — header-only C++ web framework
- **[FFmpeg](https://ffmpeg.org/)** — audio decoding
- **[Boost](https://www.boost.org/) / [Asio](https://think-async.com/Asio/)** — networking primitives

---

## 中文

### 概述

Rakuraku Music Station 是一个 C++ 流媒体广播服务器，以电台方式把同一路音频流推送给所有连接的听众。单一的 FFmpeg 解码器向无锁环形缓冲区写入数据，Linux `epoll` 循环将字节分发给每位在线听众。基于 Crow 的 Web 面板提供播放列表管理、文件上传、播控和设置功能。

### 特性

- **电台式广播** — 单一解码器、共享环形缓冲区，所有听众同步收听同一路流
- **低延迟 `epoll` I/O** — 单线程承载大量并发连接
- **多格式支持** — 通过 FFmpeg 支持 MP3、WAV、FLAC、OGG、M4A、AAC
- **热重载播放列表** — 上传后立即生效，无需重启
- **元数据 / 封面 / 歌词** — 每首曲目独立 API 接口
- **会话认证** — Cookie 管理员会话，可选开放游客切歌
- **可主题化 UI** — 台名、配色、副标题均从 `settings.json` 读取
- **PWA** — Service Worker 缓存、主题色、移动端适配
- **批量音乐下载** — 通过 `music_dl.py` 集成网易云音乐（pyncm）和 YouTube（yt-dlp）
- **网页端设置** — 在管理面板直接修改配置，无需手动编辑文件
- **一键构建** — `build_release.sh` 自动安装依赖、生成内嵌模板并生成 `dist/`

### 系统要求

- Linux（Arch / Debian / Ubuntu，构建脚本自动识别）
- 支持 C++17 的 GCC/G++ 7+
- `ffmpeg`、`openssl`、`libasio`、`wget`、`curl`、`python3`
- Python 3.8+ 并执行 `pip install -r requirements.txt`（下载功能需要）

### 快速开始

```bash
# 1. 构建
./build_release.sh

# 2. 放入音频文件
cp /path/to/music/*.mp3 dist/media/

# 3. 启动
cd dist && ./start.sh
```

浏览器访问 <http://localhost:2240>。  
音频流地址：<http://localhost:2240/stream>

使用 `./stop.sh` 停止服务。

### 手动构建（开发）

```bash
# Debian/Ubuntu 依赖
sudo apt-get install build-essential ffmpeg libssl-dev libasio-dev wget curl locales

# 下载最新版 Crow 头文件
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 调试构建
g++ radioserver.cpp metadata.cpp -o radioserver \
    -std=c++17 -g -O0 -lpthread -lssl -lcrypto -I.
```

> **`crow_all.h` 是构建前提** — 构建脚本不会自动下载。请在首次构建前手动下载（见上方 `wget` 命令）。需要更新版本时删除该文件即可。

### 配置 — `settings.json`

```json
{
    "station_name": "Rakuraku Music Station",
    "subtitle": "你的副标题",
    "primary_color": "#764ba2",
    "secondary_color": "#667eea",
    "bg_color": "#f4f4f9",
    "admin_password": "change_me",
    "allow_guest_skip": false,
    "ncm_phone": "",
    "ncm_password": "",
    "ncm_cookie": ""
}
```

| 键 | 默认值 | 说明 |
|----|--------|------|
| `admin_password` | `admin123` | **对外部署前务必修改** |
| `allow_guest_skip` | `false` | 为 `true` 时游客也可调用 `/api/next` 和 `/api/prev` |
| `ncm_*` | 空 | 网易云音乐凭据；cookie 优先于手机号+密码 |

### HTTP API

**公开接口**

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | 听众页（已登录显示管理面板） |
| GET | `/api/playlist` | 播放列表 + 元数据 |
| GET | `/api/stats` | 在线听众数 |
| GET | `/api/metadata/<idx>` | 曲目 `idx` 完整元数据 |
| GET | `/api/cover/<idx>` | 内嵌封面 |
| GET | `/api/lyrics/<idx>` | 歌词 |

**管理员接口**（需要 session cookie）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/admin/login` | `{"password":"..."}` → 设置 cookie |
| POST | `/admin/logout` | 销毁会话 |
| POST | `/upload` | 文件上传（≤ 50 MB） |
| POST | `/api/next` | 下一首 |
| POST | `/api/prev` | 上一首 |
| POST | `/api/play/<idx>` | 跳到第 `idx` 首 |
| POST | `/api/delete/<idx>` | 删除第 `idx` 首 |
| POST | `/admin/download` | 从播放列表文件批量下载 |
| GET | `/admin/download/status` | 轮询下载进度 |
| GET | `/admin/settings/get` | 读取所有设置 |
| POST | `/admin/settings/save` | 保存设置 |
| GET | `/admin/settings/ncm` | 读取网易云配置状态 |
| POST | `/admin/settings/ncm` | 保存网易云凭据 |
| POST | `/admin/settings/ncm/test` | 测试网易云登录 |

### 架构

| 组件 | 职责 |
|------|------|
| `RadioServer` | 顶层协调器 |
| `BroadcastBuffer` | 2 的幂次环形缓冲区 — 单生产者、多消费者 |
| `StreamServer` | `epoll` 分发音频字节给所有监听者 |
| `AudioPlayer` | 以 `-re` 模式启动 FFmpeg，将解码音频写入缓冲区 |
| `WebServer` | Crow 应用，提供 UI 与 REST API |
| `SessionManager` | 内存会话表，24 小时滑动过期 |
| `MetadataManager` | 通过文件名和 `ffmpeg -i` 提取元数据 |

### 项目结构

```
├── radioserver.cpp        # 主服务器（所有核心类）
├── metadata.{hpp,cpp}     # 音频元数据提取
├── sessionmanager.hpp     # 会话存储
├── authmiddleware.hpp     # 认证中间件（文档引用，尚未存在于磁盘）
├── build_release.sh       # 一键构建脚本
├── music_dl.py            # 网易云/YouTube 下载脚本
├── requirements.txt       # Python 依赖
├── manifest.json          # PWA Web Manifest
├── sw.js                  # Service Worker
├── *.html                 # HTML 模板
└── dist/                  # 构建产物（重编译时保留）
    ├── radioserver
    ├── start.sh / stop.sh
    ├── media/             # 音频文件目录
    ├── settings.json      # 运行时配置
    └── templates/         # 可选的自定义模板（覆盖内嵌版本）
```

### 故障排除

| 现象 | 解决方法 |
|------|----------|
| 非 ASCII 文件名乱码 | `sudo locale-gen zh_CN.UTF-8 && sudo update-locale LANG=zh_CN.UTF-8`，或直接使用 `./start.sh` |
| 端口被占用 | `ss -ltnp \| grep 2240`；修改 `radioserver.cpp` 中的 `Config::WEB_PORT` |
| 管道错误 `revents=16` | Locale 问题，参考上一条 |
| 没有声音 | 确认 `ffmpeg` 在 `PATH` 中，文件扩展名在支持列表内 |
| 下载失败 | 检查 Python 依赖和网易云凭据 |

日志：`dist/server.log`

### 安全建议

- 对外部署前务必修改 `admin_password`。
- 会话 Cookie 使用 `HttpOnly`、`SameSite=Lax`——公网部署请在前面挂 TLS 反向代理。
- 上传限制 50 MB 并校验扩展名。

### 许可证

MIT — 详见 [LICENSE](LICENSE)。

### 致谢

- **知夏 (Zhixia)** — 项目协作者
- **[pyncm](https://github.com/HexiNail/pyncm)** — 网易云音乐 Python API 封装
- **[Crow](https://github.com/CrowCpp/Crow)** — 轻量级 C++ Web 框架
- **[FFmpeg](https://ffmpeg.org/)** — 音频解码
- **[Boost](https://www.boost.org/) / [Asio](https://think-async.com/Asio/)** — 网络原语
- **灵感来源**：《孤独摇滚！》伊地知虹夏 — "NG"（Nijika Generation）命名由来

---

<div align="center"><em>觉得有用的话欢迎点 Star ⭐ / Star the repo if you find it useful.</em></div>
