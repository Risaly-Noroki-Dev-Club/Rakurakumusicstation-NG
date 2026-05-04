# Rakuraku Music Station NG — For long tommorow

![License](https://img.shields.io/badge/license-MIT-blue.svg) ![C++](https://img.shields.io/badge/C++-17-orange.svg) ![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

> C++ 驱动音频，Rust 掌管 Web，HTTP 对话。
> 一个自托管的网络电台，带现代 Web 界面、多用户支持、JWT 认证、WebSocket 实时同步和高性能 C++ 音频引擎。

---

## 快速开始

### 从源码构建

```bash
# 安装依赖（Debian/Ubuntu）
apt install g++ ffmpeg libssl-dev python3

# 构建
./build_release.sh

# 放入音乐文件
cp /path/to/music/*.mp3 dist/media/

# 启动（C++ 引擎 + Rust 后端）
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
| Web 界面 | `2241` | 前端 + REST API + WebSocket |
| 音频流 | `2240` | C++ 引擎推流（VLC / mpv 可直接播放 `/stream`） |
| C++ 命令 | `2240` | `POST /command` 接收 skip/prev 等指令 |
| C++ 状态 | `2240` | `GET /state` 查询当前播放状态 |

---

## 使用指南

### 首次启动

浏览器打开 `http://localhost:2241`，会出现**全屏设置向导**。输入用户名和密码，该用户即成为**电台管理员**。

后续用户可点击右上角 **登录** → 切换到 **注册** 标签创建普通账户。登录状态通过 JWT 持久化。

### 页面导航

| 标签 | 功能 |
|------|------|
| **播放器** | 当前曲目、封面、进度条、同步歌词、音量控制 |
| **队列** | 前方排队歌曲 + 播放历史 |
| **曲库** | 搜索/浏览歌曲、点歌、下载、上传、歌单、网易云账号 |

### 点歌

曲库中搜索歌曲 → 点击 **📻 点歌** 即可加入电台队列。需要登录。

### 下载歌曲

曲库列表中每首歌曲旁的 **⬇️** 按钮，点击即可下载到本地。需要登录。

### 上传歌曲

曲库页面下方的 **📤 上传歌曲** 面板，支持 MP3/FLAC/WAV/OGG/M4A/AAC（最大 100 MB）。上传后自动加入曲库。需要登录。

### 网易云账号

曲库页面下方的 **🎵 我的网易云账号** 面板，可配置个人网易云 Cookie 或手机号+密码。每个用户独立配置，互不影响。登录后可下载 VIP 歌曲。

### 播放器控制

- **播放/暂停** — 切换状态
- **音量** — 滑块调节
- **进度** — 点击/拖动进度条跳转
- **主题** — 🌓 按钮切换亮色/暗色/自动模式

### 外部播放器

VLC、mpv、ffplay 等可直接播放 `http://localhost:2240/stream`。

### PWA 支持

可在 Android/iOS 添加到主屏幕，离线缓存界面壳层。

---

## 管理员指南

管理员角色用户可看到 **管理** 标签页。

### 管理面板

| 子标签 | 功能 |
|--------|------|
| 👥 用户管理 | 查看用户列表、封禁/解封、提权/降权、操作日志 |
| 🎵 歌曲管理 | 查看/删除歌曲、重新扫描 media/、切歌 |
| 📤 上传 | 上传音频文件 |
| ⬇️ 下载 | 批量下载歌单（通过 music_dl.py） |
| 🎵 网易云 | 全局网易云账号配置 |
| ⚙️ 设置 | 电台名称、副标题、主题色、管理员密码 |
| 📊 统计 | 用户数、歌曲数、队列数、歌单数 |

### 提权用户

在 **用户管理** 中点击 **⭐ 提权** 可将普通用户提升为管理员，**⬇ 降权** 撤销。

备选方案（SQLite）：
```bash
sqlite3 dist/data/radio.db "UPDATE users SET role='admin' WHERE username='用户名';"
```
提升后需重新登录生效。

### 重新扫描媒体库

直接向 `media/` 文件夹添加文件后，在 **歌曲管理** 中点击 **🔄 重新扫描**（需服务器安装 ffprobe）。

---

## 常见问题

| 问题 | 解决方法 |
|------|----------|
| 无法连接 Web 界面 | 确认服务已启动：`cd dist && ./start.sh`；查看日志 `tail -f dist/rust-server.log` |
| 无声音/流不可用 | 确认 `media/` 中有音频文件；查看日志 `tail -f dist/server.log` |
| 曲库无歌曲 | 在管理面板点击 **重新扫描** 或重启服务 |
| 无法登录 | 首次启动通过设置向导创建管理员；普通用户通过登录弹窗注册 |
| 设置不生效 | 更改后需重启（`./stop.sh && ./start.sh`） |
| 封面不显示 | 确保音频文件含内嵌封面（ID3 标签）；缺失时显示默认音符图标 |
| 歌词不显示 | 仅支持同名 `.lrc` 文件放在同目录下 |

---

## 技术架构

```
media/  ──ffmpeg──▶  BroadcastBuffer  ──epoll──▶  N 客户端
                     (lock-free ring)   (fan-out)  (HTTP /stream)
                            ▲
                    AudioPlayer (fork ffmpeg → pipe → buffer)
                            │
                    CommandQueue (shared vector)
                            │
                    ┌───────┴────────┐
                    ▼                ▼
              POST /command    GET /state
              (Rust → C++)    (C++ → Rust)
                    ▲                │
                    │                ▼
           ┌────────┴────────────────────┐
           │  Rust Backend (2241)         │
           │  Axum + SQLite + JWT + WS   │
           │  HTTP client polls /state   │
           └─────────────────────────────┘
                            │
                    static/ (Vite-built Vue 3 SPA)
```

### 服务间通信

C++ 引擎与 Rust 后端通过 **HTTP** 通信（无 Redis）：

| 端点 | 方向 | 方法 | 说明 |
|------|------|------|------|
| `/state` | C++ → Rust | GET | Rust 每 500ms 轮询播放状态 JSON |
| `/command` | Rust → C++ | POST | Rust 发送 skip/prev/play/stop 指令 |

### 服务划分

| 服务 | 语言 | 端口 | 职责 |
|------|------|------|------|
| 音频引擎 | C++17 | 2240 | ffmpeg 解码 → 环形缓冲 → TCP 推流；HTTP 命令/状态端点 |
| 业务后端 | Rust | 2241 | REST API、WebSocket、多用户 JWT 认证、SQLite、队列管理 |

---

## API 参考

### C++ 音频引擎

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/stream` | 音频流 (`Content-Type: audio/mpeg`) |
| `GET` | `/health` | 健康检查 |
| `POST` | `/command` | 接收命令 (`{"type":"skip"}` / `"prev"` / `"stop"`) |
| `GET` | `/state` | 播放状态 JSON |

### Rust 后端

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/api/station` | 无 | 电台信息 |
| `GET` | `/api/now-playing` | 无 | 当前曲目 |
| `GET` | `/api/songs?q=` | 无 | 曲库搜索 |
| `GET` | `/api/songs/:id` | 无 | 歌曲详情 |
| `GET` | `/api/songs/:id/cover` | 无 | 封面图片 |
| `GET` | `/api/songs/:id/download` | JWT | 下载歌曲文件 |
| `POST` | `/api/songs/upload` | JWT | 上传歌曲 |
| `GET` | `/api/queue` | 无 | 队列 |
| `GET` | `/api/queue/history` | 无 | 播放历史 |
| `POST` | `/api/queue` | JWT | 点歌 |
| `POST` | `/api/auth/register` | 无 | 注册 |
| `POST` | `/api/auth/login` | 无 | 登录 |
| `GET` | `/api/auth/me` | JWT | 当前用户 |
| `GET` `POST` `DELETE` | `/api/playlists` | JWT | 歌单管理 |
| `GET` `POST` `DELETE` | `/api/favorites` | JWT | 收藏管理 |
| `GET` `POST` | `/api/ncm` | JWT | 个人网易云账号 |
| `POST` | `/api/ncm/test` | JWT | 测试网易云登录 |
| `*` | `/api/admin/*` | Admin | 管理端点 |
| `WS` | `/ws` | 无 | WebSocket 实时推送 |

---

## 配置文件 (`config.toml`)

```toml
[server]          # host, port（默认 2241）
[database]        # SQLite URL
[audio_engine]    # C++ 引擎 base_url、media_path、stream_base
[jwt]             # secret、expiry_hours
[queue]           # max_size、rate_limit
[station]         # name、subtitle、主题色
[logging]         # level
```

---

## 构建手册

```bash
# 依赖：g++、ffmpeg、libssl-dev、python3、Rust toolchain

# C++ 引擎
make            # 发布构建
make debug      # 调试构建
make clean      # 清理产物

# Rust 后端
cd radio-backend && cargo build --release

# 前端（如修改了源码）
cd radio-backend/frontend && npm install && npm run build

# 一键构建
./build_release.sh
```

支持格式：MP3、WAV、FLAC、OGG、M4A、AAC

---

## License

MIT

## Credits

- 知夏 (Zhixia) — 项目协作者
- [Crow](https://github.com/CrowCpp/Crow) — C++ HTTP 框架
- [FFmpeg](https://ffmpeg.org/) — 音频解码
- [Axum](https://github.com/tokio-rs/axum) — Rust HTTP 框架
- [Vue 3](https://vuejs.org/) — 前端框架
- [Vite](https://vitejs.dev/) — 前端构建工具
- [SQLx](https://github.com/launchbadge/sqlx) — Rust SQL 工具集

灵感来源：《孤独摇滚！》— 伊地知虹夏
