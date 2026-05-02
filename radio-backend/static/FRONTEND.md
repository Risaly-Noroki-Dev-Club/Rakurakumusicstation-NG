# Rakuraku Music Station NG — 前端设计规范 / Frontend Design Guide

> 面向接手本项目前端开发的工程师。本文档描写浏览器端 Web UI 的架构、色彩系统、API 交互约定与开发流程。
> For frontend developers joining this project. Covers the browser-side architecture, color system, API conventions, and development workflow.

---

## 文件位置 / File Map

| 文件 | 用途 | 状态 |
|------|------|------|
| `radio-backend/static/index.html` | Vue 3 模板（HTML 结构 + Vue 指令） | 活跃 |
| `radio-backend/static/style.css` | 所有 CSS 样式 | 活跃 |
| `radio-backend/static/app.js` | 所有 JavaScript 逻辑（Vue 3 组件 + 响应式 store） | 活跃 |
| `radio-backend/static/manifest.json` | PWA Web Manifest | 活跃 |
| `radio-backend/static/sw.js` | PWA Service Worker | 活跃 |

---

## 前端框架 / Frontend Framework

项目使用 **Vue 3 CDN 模式**（无构建工具）：

```html
<script src="https://unpkg.com/vue@3/dist/vue.global.prod.js"></script>
```

### 架构核心

- **响应式全局状态**：所有 UI 状态存储在 `reactive()` 单例 `store` 中，任何组件通过模板绑定自动响应变更
- **声明式渲染**：在 `index.html` 中直接使用 Vue 指令（`v-if`、`v-show`、`v-for`、`v-model`、`:class`、`:style` 等）
- **计算属性**：派生状态（进度百分比、格式化时间、歌词高亮行等）通过 `computed()` 定义，自动跟随 `playbackState` 变化
- **零构建工具**：无 webpack / vite / npm。修改 `index.html`、`app.js` 或 `style.css` 后刷新浏览器即生效

### 关键文件结构

**`app.js` — 全部前端逻辑**

```
┌─ CONSTANTS (BACKEND_URL, WS_URL, STREAM_URL, THEMES)
├─ 非响应式状态 (ws, mediaSource, sourceBuffer, 定时器)
├─ store (reactive) — 所有响应式 UI 状态
├─ 工具函数 (formatTime, escapeHtml, toast)
├─ 主题管理 (applyTheme, cycleTheme)
├─ API 辅助 (api)
├─ WebSocket (connectWebSocket, handleWsMessage)
├─ 歌词解析 (parseLyrics)
├─ HTTP 轮询 (refreshPlaybackPoll)
├─ 队列操作 (refreshQueue, refreshHistory, removeQueueItem)
├─ 认证 (doAuth, toggleAuthMode, loadCurrentUser, logout)
├─ 播放模式 (switchPlaybackMode, startFilePlayback, fetchFileChunk)
├─ 曲库搜索 (onSearchInput, addToQueue, addToMyPlaylist)
├─ 歌单 (createPlaylist)
├─ 管理功能 (用户、歌曲、上传、下载、NCM、设置、统计)
├─ Tab 切换 (switchTab, switchAdminTab)
└─ Vue App (createApp, setup, computed, mounted)
```

**`index.html` — Vue 模板**

```
<div id="app" v-cloak>
  ├─ header (站名、主题切换、用户信息、登录/退出)
  ├─ tab-bar (播放、队列、曲库、管理)
  ├─ Player Tab (封面、标题、进度条、audio、歌词)
  ├─ Queue Tab (队列列表、播放历史)
  ├─ Library Tab (搜索、歌单)
  ├─ Admin Tab (用户/歌曲/上传/下载/NCM/设置/统计)
  ├─ Auth Modal (登录/注册弹窗)
  └─ Toast Container (通知)
</div>
```

### Vue 约定

- **data → reactive store**：不再用 `let` 变量直接操作 DOM，所有状态通过 `store.xxx` 读写
- **`document.getElementById` → 模板绑定**：不再手动查 DOM，用 `v-if` / `v-show` 控制可见性，用 `ref` 拿 DOM 节点
- **`innerHTML` → 声明式渲染**：歌词、队列、用户列表等用 `v-for` 渲染，Vue 自动处理插入/更新/删除
- **添加新状态**：在 `store` 对象中加字段；需要派生计算的话在 `setup()` 中用 `computed()` 定义

---

## 色彩系统 / Color System

**原则：绝对不要在 CSS 属性或内联 style 中直接写色码。** 所有颜色必须通过 CSS 自定义属性（变量）引用。

### 可用变量

```css
:root {
    /* Rust 后端 /api/station 接口运行时注入 */
    --primary: #764ba2;          /* 主色 — 按钮、强调文字 */
    --secondary: #667eea;        /* 辅色 — 渐变、次要按钮 */
    --bg: #f4f4f9;               /* 页面背景 */

    /* 前端自行维护的语义令牌 */
    --card: #ffffff;             /* 卡片/面板背景 */
    --text: #333;                /* 正文 */
    --text-muted: #6c757d;       /* 弱化/说明 */
    --text-inverse: #ffffff;     /* 深色背景上的文字 */
    --border: #e9ecef;           /* 分割线、输入框边框 */
    --success: #28a745;
    --danger: #dc3545;
    --warning: #ffc107;
    --info: #17a2b8;

    --radius: 12px;
    --shadow: 0 4px 20px rgba(0,0,0,0.08);
}
```

### 暗色模式 / Dark Mode

三态切换（右上角 🌓），状态存 `localStorage.radio_theme`：

| 模式 | 行为 |
|------|------|
| `auto`（默认） | 跟随操作系统 `prefers-color-scheme` |
| `light` | 强制亮色 `[data-theme="light"]` |
| `dark` | 强制暗色 `[data-theme="dark"]` |

新增变量时，需要同时在三处定义默认值：`:root`、`[data-theme="dark"]`、`@media (prefers-color-scheme: dark)`。

---

## 与后端通信 / API Communication

### 端口约定

| 服务 | 端口 | 说明 |
|------|------|------|
| Rust 后端（API + WebSocket + 静态文件） | **2241** | 前端从此端口加载 |
| C++ 音频引擎（音频流） | **2240** | 无头模式 — 无 Web UI |

### 自动探测

```javascript
const BACKEND_URL = window.location.origin;
const WS_URL = (protocol === 'https:' ? 'wss://' : 'ws://') + location.host + '/ws';
```

所有 API 请求用绝对 URL（`BACKEND_URL + '/api/...'`），不要硬编码端口。

### WebSocket 消息

JSON，用 `type` 字段区分：

```json
{"type":"playback_state","song_id":1,"title":"曲目","artist":"艺人",
 "position_ms":120000,"duration_ms":240000,"lyrics_line":5,
 "lyrics_text":"[00:00.00]第一行\n[00:10.00]第二行",
 "status":"playing","cover_url":"http://...","stream_url":"http://.../stream"}

{"type":"queue_update","action":"added","song_title":"...",
 "requested_by":"...","queue_size":5}

{"type":"notice","message":"通知","level":"info"}

{"type":"ping","timestamp":123456789}
```

### HTTP API

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/api/station` | — | 电台名、主题色（JS 据此设置 CSS 变量） |
| `GET` | `/api/now-playing` | — | 当前曲目 + lyrics（HTTP 兜底轮询） |
| `GET` | `/api/songs?q=&limit=50` | — | 曲库搜索 |
| `GET` | `/api/songs/{id}/cover` | — | 封面图 |
| `GET` | `/api/queue` | — | 队列 |
| `GET` | `/api/queue/history` | — | 历史 |
| `DELETE` | `/api/queue/{id}` | Admin | 移除队列项 |
| `POST` | `/api/queue` | JWT | 点歌 |
| `POST` | `/api/auth/register` | — | 注册 |
| `POST` | `/api/auth/login` | — | 登录 → JWT |
| `GET` | `/api/auth/me` | JWT | 当前用户 |
| `GET` `POST` `DELETE` | `/api/playlists` | JWT | 歌单 |
| `*` | `/api/admin/*` | Admin | 管理操作 |

认证：注册/登录获得 JWT，存 `localStorage.radio_token`，请求头 `Authorization: Bearer <token>`。

---

## 全局状态 / Global State

使用 Vue 3 `reactive()` 单例 `store` 作为唯一状态源：

```javascript
const store = reactive({
    // 认证
    token: null, currentUser: null,

    // 播放
    playbackState: { song_id, title, artist, position_ms,
                     duration_ms, lyrics_line, status, cover_url },
    lyricsLines: [],
    useFileMode: false,
    coverLoadError: false,

    // UI
    stationName: '', activeTab: 'player', activeAdminTab: 'users',
    showAuth: false, authMode: 'login', authError: '',
    themeIdx: 0,

    // 队列 & 曲库
    queue: [], history: [],
    searchQuery: '', searchResults: [],

    // 管理面板各子模块状态
    users: [], adminLogs: [], adminSongs: [], adminStats: null,
    uploadFile: null, /* ... 上传、下载、NCM、设置状态 ... */

    // Toast 通知
    toasts: [],
});
```

两条更新路径：
1. **WebSocket**（主）：`handleWsMessage` → 更新 `store.playbackState` → Vue 自动刷新 UI
2. **HTTP 轮询**（兜底，2 秒间隔）：`GET /api/now-playing`

新增字段时，同时更新响应式 store 和两条更新路径。

---

## 管理面板 / Admin Panel

管理员（`role: "admin"` 的 JWT）可见子标签页：

| 标签 | 功能 |
|------|------|
| **用户管理** | 封禁/解封、操作日志 |
| **歌曲管理** | 列表、删除、重新扫描 media/、切歌 |
| **上传** | multipart（≤100 MB） |
| **下载** | 批量下载 via `music_dl.py` + 实时日志 |
| **网易云** | Cookie 或手机号登录 + 登录测试 |
| **设置** | 电台名、主题色、管理员密码 |
| **统计** | 用户数、歌曲数、队列数、歌单数 |

---

## 开发流程 / Workflow

```bash
cd radio-backend

# 1. 配置
cp config.toml.example config.toml

# 2. 启动 Rust 后端（需 SQLite + 可选 Redis）
cargo run
# → http://localhost:2241

# 3. 启动 C++ 音频引擎（在另一个终端）
cd ../dist && ./start.sh

# 4. （可选）启动 Redis
redis-server
```

前端使用 Vue 3 CDN，无构建工具。修改 `index.html`、`app.js` 或 `style.css` 后刷新浏览器即生效（Axum 从磁盘读取，无需重启）。

**首次加载会从 CDN 下载 Vue 3 运行时（~30KB gzip），浏览器会自动缓存。离线 PWA 能力受到 CDN 依赖影响 — 首次安装后 Service Worker 可缓存已下载的 Vue 运行时。**

### 开发规范 / Conventions

- **Vue CDN 是唯一的 CDN 依赖** — 不要引入其他外部 CDN（保持最小侵入）
- **不写裸色码** — 所有颜色通过 CSS 变量
- **不硬编码端口** — 用 `window.location.origin` 计算
- **不直接操作 DOM** — 用 Vue 响应式绑定和 `ref` 访问元素
- **`lyrics_line` + `lyrics_text` 已在服务端计算** — 前端只渲染
- **新增状态到 `store`，新增计算属性到 `setup()` 的 `computed()` 中**
- **新增样式到 `style.css`，新增逻辑到 `app.js`**
