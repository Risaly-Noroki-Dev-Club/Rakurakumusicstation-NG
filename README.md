Rakuraku Music Station 以电台的方式，将同一路音频流推送给所有连接的听众。由 `ffmpeg` 驱动的单一解码器向线程安全的环形缓冲区写入数据，Linux `epoll` 循环把字节分发到 HTTP 客户端；基于 Crow 的 Web 面板负责播放列表管理、上传与播控。

**端口配置：**
- **单端口模式**（默认）：所有服务监听第 2240 端口
- **双端口模式**：Web/API 在 2240 端口，音频流在 2241 端口

可通过 `settings.json` 中的 `separate_stream_port` 设置切换模式——无需重新编译。


## ✨ About "NG"

**"NG" stands for "Nijika Generation"** - a tribute to Nijika Ijichi from 《Bocchi the Rock!》 (孤独摇滚), reflecting the project's spirit of sharing music joyfully.

---

## English

### Overview

Rakuraku Music Station broadcasts the same audio stream to every connected listener, radio-style. A single `ffmpeg`-backed decoder feeds a thread-safe ring buffer, and a Linux `epoll` loop fans the bytes out to HTTP clients. The Crow-based web panel handles playlist management, uploads, and playback control.

**Port Configuration:**
- **Single-port mode** (default): All services listen on port 2240
- **Dual-port mode**: Web/API on 2240, audio stream on 2241

Switch modes via the `separate_stream_port` setting in `settings.json` — no recompilation needed.

### Features

- **High performance** — non-blocking I/O via Linux `epoll`; many concurrent listeners on one thread.
- **Radio-style broadcast** — a single decoder feeds a shared ring buffer; every client hears the same stream in sync.
- **Format support** — MP3, WAV, FLAC, OGG, M4A, AAC via FFmpeg.
- **Hot-reload playlist** — uploaded files appear in the playlist immediately; no restart needed.
- **Metadata / cover / lyrics APIs** — per-track endpoints for title, artist, duration, embedded cover art, and lyrics.
- **Session auth** — cookie-based admin sessions; optional guest skip permission.
- **Templated UI** — `{{VAR}}` substitution from `settings.json` (station name, colors, subtitle).
- **Mobile-friendly** — Responsive design for smartphones and tablets.
- **PWA support** — Install as a progressive web app (offline mode, home screen icon).
- **Web-based settings** — Modify configuration directly from the admin panel.
- **One-shot build** — `build_release.sh` installs deps, fetches Crow, and emits a self-contained `dist/`.
- **Batch music download** — Integrated NetEase Cloud Music and YouTube downloader via `pyncm` and `yt-dlp`.

### Requirements

- Linux (Arch, Debian, or Ubuntu — the build script auto-detects)
- GCC/G++ 7+ with C++17, or Clang
- FFmpeg, OpenSSL, Boost, Asio
- Python 3.8+ with `pip install -r requirements.txt` (for download feature)

### Quick Start

```bash
# Build — installs dependencies and produces dist/
./build_release.sh

# Add audio files (Chinese / Japanese filenames are fine)
cp /path/to/music/*.mp3 dist/media/

# Run
cd dist
./start.sh

Then open:

- Web admin: http://localhost:2240

**Audio stream**: either
- Single-port mode (default): http://localhost:2240/stream
- Dual-port mode: http://localhost:2241

To switch modes, edit `settings.json` and set `separate_stream_port` to `true` (dual-port) or `false` (single-port), then restart.

Stop with `./stop.sh`.

### Smart Build Process

The build system preserves your configuration and media files across rebuilds:
- `dist/` directory structure preserved
- `media/` folder and audio files remain intact
- `settings.json` configuration maintained
- Only the binary and scripts are updated

For fresh builds while keeping existing data:
```bash
./build_release.sh --keep-media
```

### Manual Build (Development)

```bash
# Debian/Ubuntu deps
sudo apt-get install build-essential ffmpeg libavcodec-extra libssl-dev \
                     libboost-all-dev libasio-dev wget locales

# Crow header
wget https://github.com/CrowCpp/Crow/releases/download/v1.3.2/crow_all.h

# Debug build
g++ radioserver.cpp metadata.cpp -o radioserver \
    -std=c++17 -g -O0 -lpthread -lssl -lcrypto -I.
```

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
    "separate_stream_port": false,

- `admin_password` — falls back to `admin123` if unset. **Change it in production.**
- `allow_guest_skip` — if `true`, unauthenticated clients can POST `/api/next`, `/api/prev`, and `/api/play/<idx>`.
- `ncm_*` — NetEase Cloud Music credentials for batch downloads (cookie takes precedence).

### HTTP API

Public:

| Method | Path                   | Description                                    |
| ------ | ---------------------- | ---------------------------------------------- |
| GET    | `/`                    | Listener page (or admin panel if logged in)    |
| GET    | `/api/playlist`        | Playlist + per-track metadata                  |
| GET    | `/api/stats`           | Current listener count                         |
| GET    | `/api/metadata/<idx>`  | Full metadata for track `idx`                  |
| GET    | `/api/cover/<idx>`     | Embedded cover art (or placeholder)            |
| GET    | `/api/lyrics/<idx>`    | Lyrics, if available                           |

Admin (session cookie required):

| Method | Path                 | Description                         |
| ------ | -------------------- | ----------------------------------- |
| POST   | `/admin/login`       | `{ "password": "..." }` → cookie    |
| POST   | `/admin/logout`      | Destroy session                     |
| POST   | `/upload`            | Multipart upload (≤ 50 MB)          |
| POST   | `/api/next`          | Skip forward (guest-allowed opt)    |
| POST   | `/api/prev`          | Skip backward (guest-allowed opt)   |
| POST   | `/api/play/<idx>`    | Jump to track `idx` (guest-allowed opt) |
| POST   | `/api/delete/<idx>`  | Remove track `idx`                  |
| POST   | `/admin/download`    | Batch download from playlist file   |
| GET    | `/admin/download/status` | Poll download progress             |
| POST   | `/admin/settings/ncm`      | Save NetEase credentials           |
| POST   | `/admin/settings/ncm/test` | Test NetEase login                 |
| GET    | `/admin/settings/ncm`      | Read NetEase config status         |

### Web Settings API

| Method | Path                 | Description                         |
| ------ | -------------------- | ----------------------------------- |
| GET    | `/admin/settings/get` | Read all settings (sensitive fields filtered) |
| POST   | `/admin/settings/save` | Save settings (including station name, colors, etc.) |

### Memory Release Demonstration

When releasing memory blocks, the system provides detailed information:
- **Release size**: Exact bytes being freed
- **Release strategy**: Buddy system deallocation steps
- **Resulting linked list**: Final free list structure after merge operations
- **Release principles**: Explanation of buddy system fragmentation reduction

Example output in scenario panel:
```
[释放过程] 地址: 64, 大小: 256 bytes
  1. 标记块为未分配
  2. 根据块大小找到对应级别的循环双链表
  3. 将块插入到循环双链表头部
  4. 调整指针保持循环性
  5. 检查伙伴块是否空闲, 如果是则合并

释放后形成的链表结构如下：
空闲链表 (循环双链表):
  Level 2 (256 bytes): 64 -> 192 -> (circular)
```

### Architecture

| Component          | Role                                                                  |
| ------------------ | --------------------------------------------------------------------- |
| `StreamServer`     | `epoll` loop fanning out audio bytes to listeners on port 2241        |
| `AudioPlayer`      | Spawns FFmpeg with `-re`, pipes decoded audio into the buffer         |
| `BroadcastBuffer`  | Power-of-two ring buffer; one producer, many consumers                |
| `WebServer`        | Crow app on port 2240 serving UI and REST APIs                        |
| `SessionManager`   | In-memory session table with 24-hour expiry                           |

### Ports

- `2240` — web admin / API
- `2241` — audio stream

### Project Layout

```
├── radioserver.cpp        # Main server
├── metadata.{hpp,cpp}     # Audio metadata extraction
├── sessionmanager.hpp     # Session store
├── build_release.sh       # One-shot build script
├── music_dl.py           # NetEase/YouTube download script
├── requirements.txt      # Python dependencies
├── settings.json          # Runtime config
├── manifest.json          # PWA manifest file
├── sw.js                  # Service Worker for PWA
├── templates/             # HTML templates (optional)
├── *.html                # Template files
└── dist/                  # Build output (preserved across rebuilds)
    ├── radioserver
    ├── start.sh / stop.sh
    ├── media/             # Audio files live here
    └── templates/
```

### Troubleshooting

- **Garbled non-ASCII filenames** — regenerate locale: `sudo locale-gen zh_CN.UTF-8 && sudo update-locale LANG=zh_CN.UTF-8`, or start via `./start.sh` which sets it for you.
- **Port already in use** — `ss -ltnp | grep -E ':2240|:2241'` and free the port, or edit `Config::WEB_PORT` / `Config::STREAM_PORT` in `radioserver.cpp`.
- **Pipe error `revents=16`** — usually a locale issue; see above.
- **No audio plays** — confirm `ffmpeg` is on `PATH` and the file extension is in the supported list.
- **Download fails** — check Python dependencies and NetEase credentials.

Logs live at `dist/server.log`.

### Security Notes

- Change `admin_password` before exposing the server.
- Sessions are cookie-only (`HttpOnly`, `SameSite=Lax`) — put a TLS-terminating proxy in front for public deployments.
- Uploads cap at 50 MB and reject unsupported extensions, but the admin endpoint is your trust boundary.

### 📜 License

MIT — see [LICENSE](LICENSE).

### ✨ Credits & Acknowledgments

- **Project Collaborator**: 知夏 (Zhixia) - For valuable contributions and collaboration on this project
- **[pyncm](https://github.com/HexiNail/pyncm)** - NetEase Cloud Music Python API wrapper used for batch music downloads
- **[Crow](https://github.com/CrowCpp/Crow)** — header-only C++ web framework
1. **[FFmpeg](https://ffmpeg.org/)** — audio decoding
- **[Boost](https://www.boost.org/) / [Asio](https://think-async.com/Asio/)** — networking primitives
- **Inspiration**: Nijika Ijichi from 《Bocchi the Rock!》 for the "NG" (Nijika Generation) naming

---

## 中文

### 概述

- **热重载播放列表** — 上传文件后立即出现在播放列表中，无需重启。
- **元数据 / 封面 / 歌词 API** — 按曲目提供标题、艺术家、时长、内嵌封面和歌词接口。
- **会话认证** — 基于 Cookie 的管理员会话，可选开放游客切歌权限。
- **模板化 UI** — 从 `settings.json` 注入 `{{VAR}}` 变量（台名、颜色、副标题）。
- **移动端适配** — 响应式设计，完美适应手机和平板。
- **PWA 支持** — 可作为渐进式网页应用安装（离线模式、主屏幕图标）。
- **网页端设置** — 直接在管理面板修改配置。
- **一键构建** — `build_release.sh` 自动安装依赖、拉取 Crow，并生成独立的 `dist/`。
- **批量音乐下载** — 集成网易云音乐和 YouTube 下载器，通过 `pyncm` 和 `yt-dlp` 实现。

### 系统要求

- Linux（Arch / Debian / Ubuntu，构建脚本自动识别）
- 支持 C++17 的 GCC/G++ 7+ 或 Clang
- FFmpeg、OpenSSL、Boost、Asio
- Python 3.8+ 并安装 `pip install -r requirements.txt`（用于下载功能）

### 快速开始

```bash
# 构建 — 自动安装依赖并生成 dist/
./build_release.sh

# 放入音频文件（中文 / 日文文件名都支持）
cp /path/to/music/*.mp3 dist/media/

# 启动
cd dist
./start.sh
```

访问：

- Web 管理：http://localhost:2240

**音频流**: 
- 单端口模式（默认）: http://localhost:2240/stream
- 双端口模式: http://localhost:2241

要切换模式，编辑 `settings.json` 将 `separate_stream_port` 设为 `true`（双端口）或 `false`（单端口），然后重启服务器。


### 智能构建流程

构建系统会在重编译时保留您的配置和媒体文件：
- `dist/` 目录结构被保留
- `media/` 文件夹和音频文件保持完整
- `settings.json` 配置文件维持不变
- 仅更新二进制文件和脚本

如需在保留现有数据的情况下进行全新构建：
```bash
./build_release.sh --keep-media
```

### 手动构建（开发）

```bash
# Debian/Ubuntu 依赖
sudo apt-get install build-essential ffmpeg libavcodec-extra libssl-dev \
                     libboost-all-dev libasio-dev wget locales

# 下载 Crow 头文件
wget https://github.com/CrowCpp/Crow/releases/download/v1.3.2/crow_all.h

# 调试构建
g++ radioserver.cpp metadata.cpp -o radioserver \
    -std=c++17 -g -O0 -lpthread -lssl -lcrypto -I.
```

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

- `admin_password` — 未设置时回退到 `admin123`，**生产环境务必修改**。
- `allow_guest_skip` — 为 `true` 时，未登录用户也可以 POST `/api/next`、`/api/prev` 和 `/api/play/<idx>`。
- `ncm_*` — 网易云音乐凭据，用于批量下载（cookie 优先于手机号+密码）。

### HTTP API

公开接口：

| 方法 | 路径                   | 说明                                 |
| ---- | ---------------------- | ------------------------------------ |
| GET  | `/`                    | 听众页（已登录则显示管理面板）        |
| GET  | `/api/playlist`        | 播放列表 + 每首曲目的元数据           |
| GET  | `/api/stats`           | 当前在线听众数                        |
| GET  | `/api/metadata/<idx>`  | 曲目 `idx` 的完整元数据               |
| GET  | `/api/cover/<idx>`     | 内嵌封面（无则返回占位图）            |
| GET  | `/api/lyrics/<idx>`    | 歌词（如有）                          |

管理员接口（需要 session cookie）：

| 方法 | 路径                 | 说明                              |
| ---- | -------------------- | --------------------------------- |
| POST | `/admin/login`       | `{ "password": "..." }` → cookie  |
| POST | `/admin/logout`      | 销毁会话                          |
| POST | `/upload`            | 分块上传（≤ 50 MB）               |
| POST | `/api/next`          | 下一首（可放宽给游客）            |
| POST | `/api/prev`          | 上一首（可放宽给游客）            |
| POST | `/api/play/<idx>`    | 跳转到曲目 `idx`（可放宽给游客）  |
| POST | `/api/delete/<idx>`  | 删除曲目 `idx`                    |
| POST | `/admin/download`    | 从播放列表文件批量下载            |
| GET  | `/admin/download/status` | 轮询下载进度                    |
| POST | `/admin/settings/ncm`      | 保存网易云凭据                  |
| POST | `/admin/settings/ncm/test` | 测试网易云登录                  |
| GET  | `/admin/settings/ncm`      | 读取网易云配置状态              |

### 网页设置 API

| 方法 | 路径                 | 说明                              |
| ---- | -------------------- | --------------------------------- |
| GET  | `/admin/settings/get` | 读取所有设置（敏感字段已过滤）   |
| POST | `/admin/settings/save` | 保存设置（包括台名、主题色等）   |

### 内存释放演示

释放内存块时，系统会提供详细信息：
- **释放大小**：精确的字节数
- **释放策略**：伙伴系统释放步骤说明
- **形成的链表**：合并操作后的最终空闲链表结构
- **释放原则**：伙伴系统减少碎片化的原理说明

场景面板中的示例输出：
```
[释放过程] 地址: 64, 大小: 256 bytes
  1. 标记块为未分配
  2. 根据块大小找到对应级别的循环双链表
  3. 将块插入到循环双链表头部
  4. 调整指针保持循环性
  5. 检查伙伴块是否空闲, 如果是则合并

释放后形成的链表结构如下：
空闲链表 (循环双链表):
  Level 2 (256 bytes): 64 -> 192 -> (circular)
```

### 架构

| 组件               | 职责                                                        |
| ------------------ | ----------------------------------------------------------- |
| `StreamServer`     | 使用 `epoll` 将音频字节分发到 2241 端口上的听众             |
| `AudioPlayer`      | 以 `-re` 模式调起 FFmpeg，把解码后的音频写入缓冲区          |
| `BroadcastBuffer`  | 2 的幂次容量的环形缓冲区，单生产者、多消费者                |
| `WebServer`        | 运行在 2240 端口上的 Crow 应用，提供 UI 与 REST API         |
| `SessionManager`   | 内存态会话表，24 小时过期                                   |

### 端口

- `2240` — Web 管理 / API
- `2241` — 音频流

### 项目结构

```
├── radioserver.cpp        # 主服务器
├── metadata.{hpp,cpp}     # 音频元数据提取
├── sessionmanager.hpp     # 会话存储
├── build_release.sh       # 一键构建脚本
├── music_dl.py           # 网易云/YouTube 下载脚本
├── requirements.txt      # Python 依赖
├── settings.json          # 运行时配置
├── manifest.json          # PWA 清单文件
├── sw.js                  # PWA 的 Service Worker
├── templates/             # HTML 模板（可选）
├── *.html                # 模板文件
└── dist/                  # 构建产物（重编译时保留）
    ├── radioserver
    ├── start.sh / stop.sh
    ├── media/             # 音频文件放这里
    └── templates/
```

### 故障排除

- **非 ASCII 文件名乱码** — 重新生成 locale：`sudo locale-gen zh_CN.UTF-8 && sudo update-locale LANG=zh_CN.UTF-8`，或直接使用 `./start.sh`（内部已设置）。
- **端口被占用** — `ss -ltnp | grep -E ':2240|:2241'` 查占用进程，或修改 `radioserver.cpp` 中的 `Config::WEB_PORT` / `Config::STREAM_PORT`。
- **管道错误 `revents=16`** — 通常是 locale 问题，参考上一条。
- **没有声音** — 确认 `ffmpeg` 在 `PATH` 中，且文件扩展名在支持列表内。
- **下载失败** — 检查 Python 依赖和网易云凭据。

日志位于 `dist/server.log`。

### 安全建议

- 对外部署前务必修改 `admin_password`。
- 会话仅走 Cookie（`HttpOnly`、`SameSite=Lax`），公网部署请在前面挂 TLS 反向代理。
- 上传限制 50 MB 并校验扩展名，但管理员接口本身才是信任边界。

### 📜 许可证

MIT — 详见 [LICENSE](LICENSE)。

### ✨ 致谢

- **项目协作者**：知夏 - 对本项目的宝贵贡献和协作
- **[pyncm](https://github.com/HexiNail/pyncm)** - 用于批量音乐下载的网易云音乐 Python API 封装
- **[Crow](https://github.com/CrowCpp/Crow)** — 轻量级 C++ Web 框架
- **[FFmpeg](https://ffmpeg.org/)** — 音频解码
- **[Boost](https://www.boost.org/) / [Asio](https://think-async.com/Asio/)** — 网络原语
- **灵感来源**：《孤独摇滚！》中的伊地知虹夏，为 "NG" (Nijika Generation) 命名提供了灵感

---

<div align="center"><em>如果本项目对你有帮助，欢迎点亮 ⭐ Star / Star the repo if you find it useful.</em></div>