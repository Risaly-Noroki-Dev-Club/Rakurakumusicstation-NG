# Rakuraku Music Station NG — v3 Adventure Island

![License](https://img.shields.io/badge/license-MIT-blue.svg) ![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

> Rust 全栈：嵌入式音频引擎 + Web 后端 + React 单页面前端。
> 一个自托管的网络电台，设备免密认证，WebSocket 实时同步，内嵌 Rust 音频引擎。

**v3 大版本 "Adventure Island"**

---

## 快速开始

### 一行安装（systemd）

适用于 Debian/Ubuntu、Arch Linux、Fedora 等常见 Linux 发行版。脚本会安装依赖、拉取源码、构建前端和 Rust 后端，并创建 `rakuraku-music-station` systemd 服务。

```bash
curl -fsSL https://raw.githubusercontent.com/Risaly-Noroki-Dev-Club/Rakurakumusicstation-NG/main/install.sh | sudo bash
```

安装后：

```bash
# 配置文件
sudoedit /etc/rakuraku/config.toml

# 放入音乐
sudo cp /path/to/music/*.mp3 /var/lib/rakuraku/media/

# 重启服务
sudo systemctl restart rakuraku-music-station

# 查看日志
journalctl -u rakuraku-music-station -f
```

可选环境变量：

```bash
curl -fsSL https://raw.githubusercontent.com/Risaly-Noroki-Dev-Club/Rakurakumusicstation-NG/main/install.sh \
  | sudo RAKURAKU_REF=main RAKURAKU_PORT=2241 bash
```

### 从源码构建

```bash
# 依赖 (Debian/Ubuntu)
apt install ffmpeg

# 一键构建前端 + 后端 + 发布目录（需要 Node.js ≥ 18 与 Rust toolchain）
./build_release.sh

# 放入音乐
cp /path/to/music/*.mp3 dist/media/

# 启动
cd dist && ./start.sh

# 打开 http://localhost:2241
```

### 停止服务

```bash
cd dist && ./stop.sh
```

### 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| Web 界面 + API + 音频流 + WebSocket | `2241` | 单端口，单二进制 |

---

## 构建流程（完整说明）

```
                        ┌──────────────────────────────────────────────┐
                        │                build_release.sh               │
                        │                                              │
  radio-backend/frontend│  1. npm ci            （安装依赖）            │
  ─────────────────────▶│  2. npm run build     （前端生产构建）         │
                        │     ├─ tsc --noEmit    （类型检查）           │
                        │     └─ vite build      → radio-backend/static/│
                        │                                              │
  radio-backend         │  3. cargo build --release                    │
  ─────────────────────▶│     → 单二进制（axum + 内嵌 radio-engine）    │
                        │                                              │
                        │  4. 组装 dist/                               │
                        │     ├─ 二进制（经临时文件复制，避免 ETXTBSY）  │
                        │     ├─ static/（前端产物）                    │
                        │     ├─ config.toml（仅缺失时生成）            │
                        │     ├─ start.sh / stop.sh（生成）             │
                        │     └─ 保留已有 media/ 与 data/               │
                        └──────────────────────────────────────────────┘
```

### 前端构建细节

```bash
cd radio-backend/frontend
npm ci                          # 安装依赖
npm run build                   # = tsc --noEmit + vite build
```

1. **自托管字体**：Inter Variable 与 JetBrains Mono Variable 的 woff2 子集
   随源码保存在 `src/assets/fonts/`，构建不访问外部 CDN。字体声明在
   `src/fonts.css`（`@font-face` + unicode-range），相对 `url()` 由 Vite 指纹化并加
   base 前缀，支持离线构建和子路径部署。

2. **类型检查**：`tsc --noEmit`（strict，全绿才继续）。

3. **Vite 构建**：产物输出到 `../static/`（即 `radio-backend/static/`），由后端
   `ServeDir::new("static")` 托管；`/` 未知路径回退到 `static/index.html` 进入前端路由。

### 后端构建细节

```bash
cd radio-backend && cargo build --release
```

单二进制包含：Axum 业务层 + 内嵌 `radio-engine`（ffmpeg 解码 → 环形缓冲 → `/stream`）。
运行时依赖 `ffmpeg`（播放）与 `ffprobe`（时长/封面探测，仅入库扫描与首次封面请求时调用）。

### 发布目录（dist/）

`./build_release.sh` 产出可独立运行的 `dist/`：

- `dist/rakuraku-music-station`（二进制）
- `dist/static/`（前端产物）
- `dist/config.toml`（缺失时由 `config.toml.example` 生成）
- `dist/media/`、`dist/data/`（已有数据**保留**不覆盖）
- 二进制复制经过临时文件，避免覆盖正在运行的进程时 `ETXTBSY`

### 本地开发

```bash
cd radio-backend/frontend
npm run dev          # Vite dev server :5173
```

- dev server 将 `/api`、`/ws`、`/stream`、`/manifest.json`、`/site-icon` 代理到
  后端 `localhost:2241`（后端需另行启动）。
- 代理后端可切换：`VITE_PROXY_TARGET=https://your-host npm run dev`（如远程演示站）。
- 子路径开发：`VITE_BASE_PATH=/radio/ npm run dev`，需与后端 `base_path` 一致。

---

## 使用指南

### 首次启动

浏览器打开 `http://localhost:2241`，设备自动获得 `device_token` Cookie。无需注册。

获取管理员权限：设置页 → 设备 → 盾牌图标 → 输入 `config.toml` 中的
`admin_setup_token` 提权。

### 页面导航

| 页面 | 路径 | 功能 |
|------|------|------|
| **播放器** | `/`（首页） | 封面、进度条、同步歌词、播放控制；移动端三卡滑动分页；顶部「正在收听」听众条（可展开）；右侧点歌队列（管理员可移除/排序） |
| **曲库** | `/library` | 歌曲搜索（分页加载）、本地收藏（localStorage，点击即存） |
| **设置** | `/settings` | 设备（改名/提权/退出）、外观（深浅色 + Material You 动态主题色）、桌面通知、网易云账号；管理员另有「电台管理」分区 |

### 主题

设置 → 外观：

- 主题：浅色 / 深色 / 跟随系统
- 主题色：9 个预设种子色 + 自定义色，基于 **Material You 动态取色**
  （`@material/material-color-utilities`）自动生成浅色/深色两套手调配色

### 点歌

曲库搜索 → 点击 ▶ 点歌加入队列。请求队列优先于文件夹循环，点歌后会在当前曲目结束后立即播放。

### 外部播放器

VLC、mpv、ffplay 可直接播放 `http://localhost:2241/stream`。

---

## 管理员指南

管理员在设置页看到 **电台管理** 分区（不再有独立管理页）。

### 管理面板

| 子标签 | 功能 |
|--------|------|
| 📊 概览 | 统计卡片、管理日志、播放历史 |
| 🎵 歌曲 | 歌曲列表、删除、试听、重新扫描、上传 |
| 👥 用户 | 设备列表、封禁/解封、提权/降权 |
| ⬇️ 下载 | 批量下载（粘贴链接，轮询实时进度） |
| ☁️ 网易云 | 全局网易云账号配置、歌单导入 |
| ⚙️ 电台设置 | 电台名称、副标题、描述、站点图标上传 |

播放器与点歌队列中的**切歌（上一首/下一首）**、**队列移除/排序** 按钮仅管理员可见。

### 批量下载

电台管理 → 下载，粘贴链接（每行一条），选择音质/格式，后端原生 Rust NCM 引擎批量下载。

### 重新扫描

直接放文件到 `media/` 后，电台管理 → 歌曲 → **重新扫描**。新入库歌曲会**立即后台
预热封面缓存**（ffprobe 探测 → 提取或写 missing 标记），首个封面请求直接命中缓存。

### 上传图标

电台管理 → 电台设置 → 上传站点图标（PNG/SVG/WebP/JPEG，≤ 2MB）。上传后浏览器
标签页 favicon（`/site-icon`）与 PWA manifest 图标自动跟随；未上传时使用默认电台图标。

---

## 常见问题

| 问题 | 解决方法 |
|------|----------|
| 无法连接 | 确认启动：`cd dist && ./start.sh`；查看日志 `tail -f dist/server.log` |
| 无声音 | 确认 `media/` 有音频文件；检查 `/api/station` 的 `stream_url` |
| 反代后流地址不正确 | 确保代理传递 `Host` / `X-Forwarded-*` 头，或 `stream_base` 设为绝对 URL |
| 曲库无歌曲 | 电台管理 → 重新扫描，或重启服务 |
| 无法获取管理员 | 确认 `dist/config.toml` 中 `admin_setup_token` 已设置 |
| 设置不生效 | 需重启服务 (`./stop.sh && ./start.sh`) |
| 封面不显示 | 需内嵌封面(ID3) 或同目录 `cover.jpg/png`；缺失时显示音符图标 |
| 歌词不显示 | 仅支持同名 `.lrc` 放同目录；后端自动解析推送（WS 重连后自动补发全量歌词） |
| 字体资源缺失 | 确认 `radio-backend/frontend/src/assets/fonts/` 完整后重新构建 |

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
             │  内嵌 radio-engine crate      │
             │  Axum + SQLite + Device Auth  │
             │  WebSocket + 歌词 + 队列管理   │
             └──────────────────────────────┘
                             │
                  React SPA 静态文件 (`radio-backend/static/`)
```

### 服务划分

| 组件 | 语言 | 说明 |
|------|------|------|
| 音频引擎 | Rust | `radio-engine/`，ffmpeg 解码 → 环形缓冲 → async 推流 |
| 业务后端 | Rust | `radio-backend/`，REST API、WebSocket、SQLite、静态文件服务 |
| Web 前端 | React 19 + TypeScript | `radio-backend/frontend/`，Appica UI（Base UI + Tailwind v4）组件库，zustand 状态，Vite 构建到 `radio-backend/static/` |

### 主要特性

| 特性 | 说明 |
|------|------|
| **单页面前端** | React SPA 由后端静态托管；未知路径回退 `static/index.html` 进入前端路由 |
| **实时同步** | 引擎状态 500ms 经 WebSocket 推送；切歌首帧携带全量歌词，新连接自动补发 |
| **Material You 主题** | 种子色经动态取色生成浅/深两套配色，深浅色独立适配 |
| **移动端分页** | 播放器 / 歌词 / 队列三卡滑动分页，桌面端多栏布局 |
| **本地收藏** | 收藏存 localStorage 快照，点击即存即取，无网络延迟 |
| **请求队列** | 用户点歌优先级高于文件夹循环；Wake/Notify 机制即时响应 |
| **封面缓存** | 入库即预热（ffprobe 探测 + 提取/标记），请求命中缓存；前端按 `has_cover` 门控 |
| **stream_base** | 自动检测反代(X-Forwarded-*)构建流地址，也支持相对/绝对路径 |
| **base_path** | 后端可原生挂载到子路径（如 `/radio`），前端路由、API、WebSocket、音频流同步适配 |
| **歌词预解析** | 后端 LRC → 结构化数组 WebSocket 推送，前端零解析 |
| **NCM 导入任务** | 网易云下载状态持久化到 SQLite，支持掉线恢复 |

---

## API 参考

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/` `/player` `/library` `/settings` | 无 | React SPA 入口（后端 fallback 到 `static/index.html`） |
| `GET` | `/api/station` | 无 | 电台信息（含 stream_url、ws_url、needs_setup） |
| `GET` | `/api/now-playing` | 无 | 当前曲目快照 |
| `GET` | `/api/songs?q=&limit=&offset=` | 无 | 搜索/分页歌曲 |
| `GET` | `/api/songs/:id` | 无 | 歌曲详情 |
| `GET` | `/api/songs/:id/cover` | 无 | 封面（无封面时快速返回占位 SVG） |
| `GET` | `/api/songs/:id/download` | Device | 下载歌曲 |
| `GET` | `/api/queue` | 无 | 待播队列（含真实 song id / has_cover） |
| `GET` | `/api/queue/history` | Admin | 播放历史 |
| `POST` | `/api/queue` | Device | 点歌 |
| `GET` `POST` | `/api/ncm` | Device | 个人网易云账号 |
| `POST` | `/api/auth/name` | Device | 设置/修改显示名称 |
| `POST` | `/api/auth/claim-admin` | 无 | 管理员提权(token) |
| `*` | `/api/admin/*` | Admin | 管理端点（设置/用户/歌曲/上传/下载/NCM/统计/日志） |
| `WS` | `/ws` | Device (cookie) | WebSocket 实时推送（回 `"pong"` 保活） |
| `GET` | `/stream` | 无 | 音频流 (`audio/mpeg`，外部播放器可直接播放) |
| `GET` | `/manifest.json` `/site-icon` | 无 | PWA manifest / 站点图标（未上传时返回默认 SVG） |

`/api/now-playing.position_ms` 是电台引擎进度，保留给外部集成兼容使用。播放历史和真实点歌人只对管理员开放。

---

## 配置文件 (`config.toml`)

```toml
[server]          # host、port（默认 2241）、base_path（默认 /）
[database]        # SQLite URL
[audio_engine]    # media_path、stream_base（auto / 相对路径 / 绝对 URL）
[device]          # cookie_max_age_days、admin_setup_token
[queue]           # max_size、rate_limit
[station]         # name、short_name、subtitle、description、icon_path
[logging]         # level
[ncm]             # device_id、download_concurrency
```

所有配置均可通过同名环境变量覆盖（`RADIO_*` 前缀），见 `radio-backend/.env.example`。

### 子路径部署 (`base_path`)

默认根路径部署无需额外配置：

```toml
[server]
base_path = "/"
```

如果要部署在 `https://example.com/radio/`，后端配置和前端构建路径必须一致：

```toml
[server]
base_path = "/radio"
```

```bash
cd radio-backend/frontend
VITE_BASE_PATH=/radio/ npm run build
```

此模式下后端原生服务 `/radio/`、`/radio/api/*`、`/radio/ws`、`/radio/stream`，反向代理应保留 `/radio` 前缀转发给后端，不要剥离前缀。

### `stream_base` 三种模式

| 值 | 场景 |
|----|------|
| `"auto"` | **推荐。** 自动根据 `Host` / `X-Forwarded-*` 请求头推断流地址，适用于大多数部署（含反向代理）。 |
| `"/stream"` | 相对路径。前端用 `window.location.origin` 解析，适用于简单的内网直连。 |
| `"http://cdn.example.com/stream"` | 强制绝对 URL。适用于 CDN 或独立流服务器。 |

当 `stream_base = "auto"` 或 `stream_base = "/stream"` 时，后端会自动叠加 `server.base_path`。

### PWA

前端支持根路径和子路径 PWA：`manifest.json` 由后端动态生成（含站点图标）。
反代 HTTPS 域名下应确认：

- `/manifest.json` 或 `<base_path>/manifest.json` 返回 JSON。
- `icon.svg`、`/site-icon` 可访问（未上传站点图标时返回默认 SVG）。

如果更换 `base_path`，必须重新运行对应 `VITE_BASE_PATH` 的前端构建。

---

## 构建手册

```bash
# 依赖: ffmpeg, Node.js ≥ 18, Rust toolchain

# 一键完整发布（前端 + 后端 + dist/ 组装）
./build_release.sh
# 跳过前端（仅编译后端 + 组装）
./build_release.sh --skip-frontend

# 仅前端（更新 radio-backend/static/）
cd radio-backend/frontend && npm run build

# 仅后端
cd radio-backend && cargo build --release

# 引擎单元测试
cd radio-engine && cargo test ring_buffer
```

支持格式: MP3、WAV、FLAC、OGG、M4A、AAC

---

## License

MIT

本项目包含以下第三方代码：

- **网易云下载模块** (`radio-backend/src/services/ncm/`) 的 Eapi 加密逻辑与 API 调用参考自 [Music163bot-Go](https://github.com/XiaoMengXinX/Music163bot-Go)（GPL-3.0 许可证）。相关代码在 Rust 中进行了重写和适配。

## Credits

- 知夏 (Zhixia) — 项目协作者
- [FFmpeg](https://ffmpeg.org/) — 音频解码
- [Axum](https://github.com/tokio-rs/axum) — Rust HTTP 框架
- [React](https://react.dev/) / [Appica UI](https://appica.dev/) / [Vite](https://vite.dev/) — Web 前端
- [Material Color Utilities](https://github.com/material-foundation/material-color-utilities) — Material You 动态取色
- [SQLx](https://github.com/launchbadge/sqlx) — Rust SQL 工具集
- [Music163bot-Go](https://github.com/XiaoMengXinX/Music163bot-Go) — 网易云 API 参考 (GPL-3.0)

灵感来源: 《孤独摇滚！》— 伊地知虹夏

---

## 人生致谢

Chinese Football 在《Win&Lose》的封底写过：

> 每个人都想成为赢家，想让自己付出的时间得到胜利的喜悦作为回报。
>
> 日复一日，我开始接受自己是一个失败者，也开始接受有些梦想注定会失败这个事实。我学会安慰自己：你拥有的是过程，至少你尝试过，收获在别处，你已经赢下了与自己的战斗。
>
> 那么就祝贺自己还算清醒吧。我没有在与他人竞争之后迷失于虚荣，也没有在与自己竞争之后沉溺于情绪。
>
> 只是我有时仍然会做梦，在其中一个梦里，我还没有抵达最终的结局。在某一个结局里，我最终成为了一个强大的人，而 Chinese Football 成为了中国摇滚的传奇。

对我来说，这个项目大概也是这样的心情。

快要十八岁了，我还不是一个厉害的大人，也不敢说自己真的多么会写代码。这个项目里有许多求助、试错、重写、妥协和大模型留下的痕迹。也许从传统意义上说，它并不是一个人独自完成的胜利。

但收获在别处。

至少我认真地想过自己想做什么，至少我把一个想法从混乱带到了可以运行、可以使用、可以告一段落的地方。至少我在怀疑自己的时候还是继续往前推了一点。至少在这个版本结束的时候，我可以承认：我没有真正成为某种意义上的赢家，但我也没有输给自己。

于是把这段话留在这里，当作这个项目的封底，也当作一份人生致谢。

我想特别感谢我的家人。他们一直给我前行的勇气，也是我成长的底气。

我想感谢知夏、噗噗砰砰砰、lunatic、violet、鹤汣以及成玉河，感谢他们在千里之外的陪伴，排名不分先后。

我还想感谢雕佬，谢谢你请我吃了这么多餐，你简直是个天才。

我还想感谢 Chinese Football 乐队。下一个十年，我们一起冲出亚洲，走向世界！

最后，我想感谢 Cynun。谢谢 ta 让我重拾这份很久之前的计划。我们一直都在。

献给那些最终没有完全实现、但仍然照亮过我的梦。
