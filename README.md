# Rakuraku Music Station NG — v3 Adventure Island

![License](https://img.shields.io/badge/license-MIT-blue.svg) ![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

> Rust 全栈：嵌入式音频引擎 + Web 后端 + Vue 单页面前端。
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

# 构建前端静态文件 (需要 Node.js)
cd radio-backend/frontend && npm run build
cd ../..

# 构建发布包 (需要 Rust toolchain)
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

## 使用指南

### 首次启动

浏览器打开 `http://localhost:2241`，设备自动获得 `device_token` Cookie。无需注册。

获取管理员权限：在「设置」页面输入 `config.toml` 中的 `admin_setup_token` 申请提权。

### 页面导航

| 页面 | 路径 | 功能 |
|------|------|------|
| **播放器** | `/` | 封面、进度条、同步歌词、播放控制 |
| **曲库** | `/library` | 搜索歌曲、点歌、上传、歌单、网易云账号绑定 |
| **队列** | `/up-next` | 待播队列；管理员可查看播放历史；`/queue` 会兼容跳转到此页 |
| **设置** | `/settings` | 显示名称、亮色/暗色/自动主题、管理员提权 |

页面顶部有音频进度条常驻，曲目封面、歌词通过 WebSocket 实时同步。

### 点歌

曲库搜索 → 点击 **📻 点歌** 加入队列。请求队列优先于文件夹循环，点歌后会在当前曲目结束后立即播放。

### 上传歌曲

曲库页面下方 **⬆️ 上传歌曲**，支持 MP3 / FLAC / WAV / OGG / M4A / AAC（最大 100 MB）。上传后自动加入曲库并刷新播放队列。

### 网易云账号

曲库页面下方 **☁️ 网易云账号**，可填入个人网易云 Cookie 或手机号 + 密码。每设备独立存储，可下载 VIP 歌曲。

### 外部播放器

VLC、mpv、ffplay 可直接播放 `http://localhost:2241/stream`。

---

## 管理员指南

管理员在侧栏看到 **🛡️ 管理** 入口。

### 管理面板

| 子标签 | 功能 |
|--------|------|
| 📊 统计 | 用户数、歌曲数、队列数、歌单数 |
| 👥 用户 | 设备列表、封禁/解封、提权/降权、操作日志 |
| 🎵 歌曲 | 歌曲列表、删除、重新扫描、上一首/下一首 |
| ⬆️ 上传 | 上传音频文件 |
| ⬇️ 下载 | 批量网易云下载（粘贴歌单，SSE 实时日志） |
| ☁️ 网易云 | 全局网易云账号配置 |
| ⚙️ 设置 | 电台名称、副标题、主题色 |

### 批量下载

管理 → 下载，输入歌单（每行 `艺术家 - 歌名`），选择音质和格式，即可通过原生 Rust NCM 引擎批量下载。进度通过 SSE 实时推送到页面。

### 提权设备

用户管理 → 点击 **提权** 升级为管理员；**降权** 撤销。

SQL 备选方案：
```bash
sqlite3 dist/data/radio.db "UPDATE device_users SET role='admin' WHERE id=设备ID;"
```

### 重新扫描

直接放文件到 `media/` 后，歌曲管理中点击 **🔄 重新扫描**（需要 ffprobe）。

---

## 常见问题

| 问题 | 解决方法 |
|------|----------|
| 无法连接 | 确认启动：`cd dist && ./start.sh`；查看日志 `tail -f dist/server.log` |
| 无声音 | 确认 `media/` 有音频文件；检查 `/api/station` 的 `stream_url` |
| 反代后流地址不正确 | 确保代理传递 `Host` / `X-Forwarded-*` 头，或 `stream_base` 设为绝对 URL |
| 曲库无歌曲 | 管理面板点击 **重新扫描**，或重启服务 |
| 无法获取管理员 | 确认 `dist/config.toml` 中 `admin_setup_token` 已设置 |
| 设置不生效 | 需重启服务 (`./stop.sh && ./start.sh`) |
| 封面不显示 | 需内嵌封面(ID3)；缺失时显示默认音符图标 |
| 歌词不显示 | 仅支持同名 `.lrc` 放同目录；后端自动解析推送 |

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
                      Vue SPA 静态文件 (`radio-backend/static/`)
```

### 服务划分

| 组件 | 语言 | 说明 |
|------|------|------|
| 音频引擎 | Rust | `radio-engine/`，ffmpeg 解码 → 环形缓冲 → async 推流 |
| 业务后端 | Rust | `radio-backend/`，REST API、WebSocket、SQLite、静态文件服务 |
| Web 前端 | Vue 3 + TypeScript | `radio-backend/frontend/`，Vite 构建到 `radio-backend/static/` |

### v3.0.0 主要特性

| 特性 | 说明 |
|------|------|
| **单页面前端** | Vue SPA 由后端静态托管；页面路径通过 `static/index.html` fallback 进入前端路由 |
| **公开曲库** | 曲库页默认展示全站歌曲、总数和分页加载，搜索复用同一公开接口 |
| **主题模式** | 支持自动/浅色/深色，自动模式跟随系统主题并同步 Vuetify 与自定义 CSS 变量 |
| **非线性动效** | 列表、迷你播放器、背景和主题切换使用强调曲线动效，并尊重 reduced-motion |
| **请求队列** | 用户点歌优先级高于文件夹循环；Wake/Notify 机制即时响应 |
| **批量下载** | 管理面板粘贴歌单批量下载，SSE 推送实时进度 |
| **stream_base** | 自动检测反代(X-Forwarded-*)构建流地址，也支持相对/绝对路径 |
| **base_path** | 后端可原生挂载到子路径（如 `/radio`），前端路由、PWA、API、WebSocket、音频流同步适配 |
| **路径标准化** | engine 内部统一相对路径存储，`resolve_media_path` 处理绝对/相对 |
| **歌词预解析** | 后端 LRC → 结构化数组 WebSocket 推送，前端零解析 |
| **NCM 导入任务** | 网易云下载状态持久化到 SQLite，支持掉线恢复 |

---

## API 参考

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/` `/library` `/up-next` `/queue` `/settings` `/admin/*` | 无 | Vue SPA 入口（`/queue` 兼容跳转到 `/up-next`） |
| `GET` | `/api/station` | 无 | 电台信息 |
| `GET` | `/api/now-playing` | 无 | 当前曲目 |
| `GET` | `/api/songs?q=` | 无 | 搜索歌曲 |
| `GET` | `/api/songs/:id` | 无 | 歌曲详情 |
| `GET` | `/api/songs/:id/cover` | 无 | 封面图片 |
| `GET` | `/api/songs/:id/download` | Device | 下载歌曲 |
| `POST` | `/api/songs/upload` | Device | 上传歌曲 |
| `GET` | `/api/queue` | 无 | 待播队列；普通用户不返回真实点歌人 |
| `GET` | `/api/queue/history` | Admin | 播放历史 |
| `POST` | `/api/queue` | Device | 点歌 |
| `GET` `POST` `DELETE` | `/api/playlists` | Device | 歌单管理 |
| `GET` `POST` | `/api/ncm` | Device | 个人网易云账号 |
| `POST` | `/api/ncm/test` | Device | 测试网易云登录 |
| `POST` | `/api/auth/name` | Device | 修改显示名称 |
| `POST` | `/api/auth/claim-admin` | 无 | 管理员提权(token) |
| `*` | `/api/admin/*` | Admin | 管理端点 |
| `WS` | `/ws` | Device (query) | WebSocket 实时推送 |
| `GET` | `/stream` | 无 | 音频流 (`audio/mpeg`) |

`/api/now-playing.position_ms` 是电台引擎进度，保留给外部集成兼容使用。官方前端不会用它暴露用户侧的点歌历史。播放历史和真实点歌人只对管理员开放。

---

## 配置文件 (`config.toml`)

```toml
[server]          # host、port（默认 2241）、base_path（默认 /）
[database]        # SQLite URL
[audio_engine]    # media_path、stream_base（auto / 相对路径 / 绝对 URL）
[device]          # cookie_max_age_days、admin_setup_token
[queue]           # max_size、rate_limit
[station]         # name、subtitle、主题色
[logging]         # level
```

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

前端已支持根路径和子路径 PWA：`manifest.json`、`sw.js`、service worker scope、Vue Router base 都由 `VITE_BASE_PATH` 控制。反代 HTTPS 域名下应确认：

- `/manifest.json` 或 `<base_path>/manifest.json` 返回 JSON。
- `/sw.js` 或 `<base_path>/sw.js` 返回 JavaScript，不应被反代改写成 HTML。
- `icon.svg`、`icon-192.png`、`icon-512.png` 可访问。
- 如果更换 `base_path`，必须重新运行对应 `VITE_BASE_PATH` 的前端构建。

---

## 构建手册

```bash
# 依赖: ffmpeg, Rust toolchain

# 前端生产构建（会更新 radio-backend/static/）
cd radio-backend/frontend && npm run build
cd ../..

# 子路径部署示例（需与 [server].base_path 一致）
cd radio-backend/frontend && VITE_BASE_PATH=/radio/ npm run build
cd ../..

# 一键打包发布目录（复制现有 static/，不会自动运行 Vite）
./build_release.sh

# 仅 Rust
cd radio-backend && cargo build --release
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
- [Vue](https://vuejs.org/) / [Vite](https://vite.dev/) — Web 前端
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
