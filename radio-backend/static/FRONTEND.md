# Rakuraku Music Station NG — 前端设计规范

> 面向接手本项目前端开发的工程师。本文档描述浏览器端 Web UI 的架构、色彩系统、API 交互约定和开发流程。

---

## 文件位置

| 文件 | 用途 | 状态 |
|------|------|------|
| `radio-backend/static/index.html` | 主力播放器 UI（HTML 结构） | **活跃开发** |
| `radio-backend/static/style.css` | 所有 CSS 样式（从 index.html 拆分） | **活跃开发** |
| `radio-backend/static/app.js` | 所有 JavaScript（从 index.html 拆分） | **活跃开发** |
| `radio-backend/static/manifest.json` | PWA Web Manifest | 公用 |
| `radio-backend/static/sw.js` | PWA Service Worker | 公用 |
| `index.html` | C++ Crow 服务器听众页模板 | 遗留 |
| `panel.html` | C++ Crow 管理面板模板 | 遗留 |
| `login.html` | C++ Crow 登录页模板 | 遗留 |

C++ 模板的功能正在逐步迁移到 Rust 后端的前端中，后续 `index.html` / `panel.html` / `login.html` 将被废弃。

---

## 色彩系统

### 原则

**绝对不要在 CSS 属性或内联 style 中直接写色码（如 `#e9ecef`、`#fff`、`#333`）。**
所有颜色通过 CSS 自定义属性（变量）引用，在 `:root` 中统一定义。

### 可用变量

```css
:root {
    /* 由后端 /api/station 接口在 JS 中动态注入 */
    --primary: #764ba2;       /* 主色（按钮、强调文字） */
    --secondary: #667eea;     /* 辅色（渐变、次要按钮） */
    --bg: #f4f4f9;            /* 页面背景色 */

    /* 前端自行维护的语义令牌 */
    --card: #ffffff;          /* 卡片/面板背景 */
    --text: #333;             /* 正文文字 */
    --text-muted: #6c757d;    /* 弱化/说明文字 */
    --text-inverse: #ffffff;  /* 深色背景上的文字（按钮、Badge、Toast） */
    --border: #e9ecef;        /* 输入框边框、分割线、进度条底色 */
    --success: #28a745;       /* 成功/绿色 */
    --danger: #dc3545;        /* 危险/红色 */
    --warning: #ffc107;       /* 警告/黄色 */
    --info: #17a2b8;          /* 信息/青色 */

    --radius: 12px;           /* 统一圆角 */
    --shadow: 0 4px 20px rgba(0,0,0,0.08); /* 卡片阴影 */
}
```

### 暗色模式

前端实现了三态主题切换：

| 模式 | 行为 |
|------|------|
| `auto`（默认） | 跟随操作系统 `prefers-color-scheme` 媒体查询 |
| `light` | 强制亮色（`:root[data-theme="light"]`） |
| `dark` | 强制暗色（`:root[data-theme="dark"]`） |

切换按钮在页面右上角 🌓，状态持久化到 `localStorage.radio_theme`。

### 新增变量时

在 `:root` 中添加默认值，并在 `:root[data-theme="dark"]` 和 `@media (prefers-color-scheme: dark)` 中添加暗色对应值。不要给 CSS 属性写裸色码。

---

## 与后端通信

### 端口约定

| 服务 | 默认端口 | 说明 |
|------|---------|------|
| Rust 后端（API + WebSocket + 静态文件） | **2241** | 前端由此后端托管并从此端口加载 |
| C++ 音频引擎（推流 / 媒体文件） | **2240** | 独立的 legacy 服务 |

### URL 配置

前端 JS 启动时自动检测：

```javascript
const BACKEND_URL = window.location.origin;       // 自动带端口
const AUDIO_ENGINE_URL = protocol + '//' + hostname + ':2240';
const WS_URL = (protocol === 'https:' ? 'wss://' : 'ws://') + location.host + '/ws';
```

所有 API 请求使用绝对 URL（`BACKEND_URL + '/api/...'`），**不要**写死端口号。

### WebSocket 消息格式

后端通过 WebSocket 推送 JSON，以 `type` 字段区分消息类型：

```json
{
  "type": "playback_state",
  "song_id": 1,
  "title": "曲目名称",
  "artist": "艺术家",
  "position_ms": 120000,
  "duration_ms": 240000,
  "lyrics_line": 5,
  "lyrics_text": "[00:00.00]第一行歌词\n[00:10.00]第二行\n",
  "status": "playing"
}
```

**`lyrics_line` 和 `lyrics_text` 已由后端服务端计算并注入。** 前端只需调用 `parseLyrics(lyrics_text)` 渲染歌词列表，不再需要自行解析 LRC 格式。

### 其他 WS 消息类型

```json
{"type":"queue_update","action":"added","song_title":"...","requested_by":"...","queue_size":5}
{"type":"notice","message":"通知文本","level":"info"}
{"type":"ping","timestamp":123456789}
```

### HTTP API（Rust 后端，端口 2241）

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/api/station` | 否 | 电台名称、主题色（JS 据此设置 `--primary` / `--secondary` / `--bg`） |
| `GET` | `/api/now-playing` | 否 | 当前曲目 + `lyrics_text` / `lyrics_line`（HTTP 轮询兜底） |
| `GET` | `/api/songs?q=&limit=20` | 否 | 曲库搜索 |
| `GET` | `/api/songs/{id}/cover` | 否 | 封面图片（JPEG/PNG 二进制） |
| `GET` | `/api/queue` | 否 | 公共队列 |
| `GET` | `/api/queue/history` | 否 | 播放历史 |
| `POST` | `/api/queue` | 是 | 点歌（需 Bearer token） |
| `POST` | `/api/auth/register` | 否 | 注册 |
| `POST` | `/api/auth/login` | 否 | 登录 |
| `GET` | `/api/auth/me` | 是 | 当前用户信息 |
| `GET` | `/api/favorites` | 是 | 收藏歌曲列表 |
| `POST` | `/api/favorites/{song_id}` | 是 | 收藏 |
| `DELETE` | `/api/favorites/{song_id}` | 是 | 取消收藏 |

认证方式：注册/登录后获得 JWT，放在 `Authorization: Bearer <token>` 头中，前端将 token 存在 `localStorage.radio_token`。

---

## 全局状态

前端使用纯 JavaScript（无框架），全局 `playbackState` 对象作为唯一状态源：

```javascript
let playbackState = {
    song_id: 0,
    title: '',
    artist: '',
    position_ms: 0,
    duration_ms: 0,
    lyrics_line: null,
    status: 'stopped',
    cover_url: ''
};
```

状态通过两条路径更新：
1. **WebSocket**（主路径）：`handleWsMessage` → 更新 `playbackState` → 刷新 UI
2. **HTTP 轮询**（兜底，2 秒间隔）：`refreshPlaybackPoll` → `GET /api/now-playing` → 同上

新增字段时，需同时更新两条路径的赋值逻辑。

---

## 开发流程

```bash
cd radio-backend

# 启动后端（需要 SQLite + Redis）
cp config.toml.example config.toml
cargo run

# 打开浏览器访问 http://localhost:2241
# 前端文件位于 static/index.html，修改后刷新浏览器即时生效
# （Axum 从磁盘读取静态文件，无需重启）
```

### 注意

- 前端不依赖任何构建工具（无 webpack / vite / npm）
- CSS 在 `style.css` 中，JS 在 `app.js` 中，HTML 结构在 `index.html` 中
- **不要**引入外部 CDN 资源（会使离线 PWA 场景失效）
- 新增样式时，在 `style.css` 中添加；新增逻辑时，在 `app.js` 中添加
- 所有颜色必须通过 CSS 自定义属性（变量）引用，禁止裸色码

### 管理面板（Admin Panel）

管理员登录后，管理面板提供以下子标签页：
- **用户管理** — 查看/封禁/解封用户，操作日志
- **歌曲管理** — 查看所有歌曲，删除歌曲，重新扫描媒体目录，播放控制
- **上传** — 上传 MP3/WAV/FLAC/OGG/M4A/AAC 文件到媒体目录
- **下载** — 批量下载歌单（通过 music_dl.py）
- **网易云** — 配置网易云音乐 Cookie 或手机号登录
- **设置** — 电台名称、主题颜色、管理员密码
- **统计** — 用户数、歌曲数、队列数、歌单数

---

## 维护清单

- [x] `panel.html` 的上传、下载、设置、网易云功能已移植到 Rust 前端
- [x] `radio-backend/static/index.html` 已拆分为 `index.html` + `style.css` + `app.js`
- [ ] C++ 模板（`index.html` / `panel.html` / `login.html`）在下一次主要发布时可删除
- [ ] `allow_guest_skip` 功能待实现（需在配置文件中新增字段）
