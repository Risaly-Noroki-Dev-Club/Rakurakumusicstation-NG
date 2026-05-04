# Rakuraku Music Station NG — 前端设计规范 / Frontend Design Guide

> 面向接手本项目前端开发的工程师。本文档描写浏览器端 Web UI 的架构、色彩系统、API 交互约定与开发流程。

---

## 文件位置 / File Map

| 文件 | 用途 | 状态 |
|------|------|------|
| `radio-backend/frontend/` | Vite + Vue 3 + TypeScript SFC 项目根目录 | 活跃 |
| `radio-backend/frontend/src/main.ts` | Vue 应用入口 | 活跃 |
| `radio-backend/frontend/src/App.vue` | 根组件（布局 + router-view + 全局 overlay） | 活跃 |
| `radio-backend/frontend/src/router.ts` | vue-router 路由配置（history 模式） | 活跃 |
| `radio-backend/frontend/src/store.ts` | 响应式全局 store + 工具函数 | 活跃 |
| `radio-backend/frontend/src/api.ts` | API 层（HTTP + WebSocket + 轮询） | 活跃 |
| `radio-backend/frontend/src/types.ts` | TypeScript 类型定义 | 活跃 |
| `radio-backend/frontend/src/style.css` | 全局 CSS 样式 + CSS 变量主题 | 活跃 |
| `radio-backend/frontend/src/views/` | 页面级组件 | 活跃 |
| `radio-backend/frontend/src/components/` | 共享组件 + admin 子组件 | 活跃 |
| `radio-backend/frontend/src/components/admin/` | 管理面板 7 个子组件 | 活跃 |
| `radio-backend/frontend/public/` | 直接复制的静态文件（manifest.json, sw.js） | 活跃 |
| `radio-backend/static/` | **构建产物**目录（Vite 输出 + ServeDir 目标） | 构建生成 |
| `radio-backend/static/assets/` | 带哈希的 JS/CSS 产物 | 构建生成 |

---

## 前端框架 / Frontend Framework

项目使用 **Vue 3 SFC + Vite + TypeScript**：

| 依赖 | 版本 | 用途 |
|------|------|------|
| vue | ^3.5 | 前端框架 |
| vue-router | ^4.5 | SPA 路由（history 模式） |
| vite | ^6.2 | 构建工具 + 开发服务器 |
| typescript | ~5.7 | 类型系统 |
| vue-tsc | ^2.2 | Vue SFC 类型检查 |
| @vitejs/plugin-vue | ^5.2 | Vite Vue SFC 插件 |

### 架构核心

- **SFC 组件**：每个功能模块是独立的 `.vue` 单文件组件（template + script + style）
- **响应式全局状态**：所有 UI 状态存储在 `reactive()` 单例 `store` 中（`src/store.ts`）
- **类型安全**：所有状态、API 响应、WebSocket 消息均有 TypeScript 接口定义（`src/types.ts`）
- **vue-router history 模式**：URL 驱动导航（`/player`, `/queue`, `/library`, `/admin/:subtab`）
- **Rust 后端 SPA 回退**：`ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"))`

### 关键文件结构

**`src/main.ts`** — 应用入口
```
import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import './style.css'
createApp(App).use(router).mount('#app')
```

**`src/App.vue`** — 根组件
- Setup overlay（首次运行创建管理员）
- Normal app layout：HeaderBar + NavTabs + `<router-view>` + AuthModal + ToastContainer
- 生命周期：loadStationInfo → connectWebSocket → 认证 → 轮询

**`src/store.ts`** — 全局状态
```
reactive({
  token, currentUser, stationName, needsSetup,
  playbackState, lyricsLines, useFileMode,
  queue, history, searchQuery, searchResults,
  myPlaylists, toasts,
  // ... 管理面板状态
})
```
导出工具函数：`formatTime`, `toast`, `applyTheme`, `cycleTheme`, `applyStationColors`, `parseLyrics`

**`src/api.ts`** — API 层
- HTTP：fetch + JWT auth headers
- WebSocket：自动重连（指数退避，最多 20 次）
- 轮询：队列 5s、播放状态 2s（WebSocket 断开时兜底）
- 播放控制：推流模式 / 推文件模式切换

**`src/router.ts`** — 路由配置
```
/            → redirect /player
/player      → PlayerView
/queue       → QueueView
/library     → LibraryView
/admin       → redirect /admin/users
/admin/:subtab → AdminView (meta: requiresAdmin)
```
导航守卫：`/admin/*` 需要 `role === 'admin'`，否则重定向到 `/player`。

### 组件树 / Component Tree

```
App.vue
├── HeaderBar.vue              # 站名、主题切换、用户信息、登录/退出
├── NavTabs.vue                # 导航标签栏（router-link）
├── <router-view>
│   ├── PlayerView.vue         # 播放页面（封面、进度条、audio、歌词）
│   ├── QueueView.vue          # 队列 + 播放历史
│   ├── LibraryView.vue        # 曲库搜索 + 我的歌单 + NCM 用户配置
│   └── AdminView.vue          # 管理页面壳（router-view 分配子标签）
│       ├── AdminUsers.vue     # 👥 用户管理（提权/降权/封禁/日志）
│       ├── AdminSongs.vue     # 🎵 歌曲管理（删除/重新扫描/切歌）
│       ├── AdminUpload.vue    # 📤 上传
│       ├── AdminDownload.vue  # ⬇️ 批量下载
│       ├── AdminNcm.vue       # 🎵 网易云（复用 NcmSettings 组件）
│       ├── AdminSettings.vue  # ⚙️ 系统设置
│       └── AdminStats.vue     # 📊 统计
├── AuthModal.vue              # 登录/注册弹窗
├── ToastContainer.vue         # 通知提示
├── StatusMessage.vue          # 通用状态提示（替换内联样式）
└── NcmSettings.vue            # 网易云配置（管理员 + 用户复用）
```

---

## 色彩系统 / Color System

所有颜色通过 CSS 自定义属性（变量）引用。

```css
:root {
    --primary: #764ba2;
    --secondary: #667eea;
    --bg: #f4f4f9;
    --card: #ffffff;
    --text: #333;
    --text-muted: #6c757d;
    --text-inverse: #ffffff;
    --border: #e9ecef;
    --success: #28a745;
    --danger: #dc3545;
    --warning: #ffc107;
    --info: #17a2b8;
    --radius: 12px;
    --shadow: 0 4px 20px rgba(0,0,0,0.08);
}
```

`--primary`, `--secondary`, `--bg` 在运行时由 `/api/station` 接口动态覆盖。

---

## 与后端通信 / API Communication

### 端口约定

| 服务 | 端口 | 说明 |
|------|------|------|
| Rust 后端（API + WebSocket + 静态文件） | **2241** | 前端从此端口加载 |
| C++ 音频引擎（音频流 + 命令 + 状态） | **2240** | 无头模式 |
| Vite 开发服务器 | **5173** | 仅开发时使用，代理 API 到 2241 |

### 开发代理

`vite.config.ts` 中的 dev server 代理配置：
```ts
proxy: {
  '/api': 'http://localhost:2241',
  '/ws': { target: 'ws://localhost:2241', ws: true },
  '/stream': 'http://localhost:2240',
}
```

### WebSocket 消息

JSON，用 `type` 字段区分：`playback_state`, `queue_update`, `notice`, `ping`

### HTTP API

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| `GET` | `/api/station` | — | 电台名、主题色 |
| `GET` | `/api/now-playing` | — | 当前曲目（HTTP 兜底） |
| `GET` | `/api/songs?q=&limit=50` | — | 曲库搜索 |
| `POST` | `/api/auth/register` | — | 注册 |
| `POST` | `/api/auth/login` | — | 登录 → JWT |
| `GET` `POST` | `/api/queue` | JWT/— | 队列 |
| `GET` `POST` `DELETE` | `/api/playlists` | JWT | 歌单 |
| `*` | `/api/admin/*` | Admin | 管理操作 |

---

## 开发流程 / Workflow

```bash
cd radio-backend/frontend

# 开发模式（热更新代理到 Rust 后端）
npm run dev
# → http://localhost:5173

# 生产构建（输出到 ../static/）
npm run build
# → 生成 radio-backend/static/index.html + assets/*

# Rust 后端构建
cd .. && cargo build
# 或从项目根目录
./build_release.sh
```

### 构建集成

`vite.config.ts` 配置：
- `build.outDir: '../static'` — 输出到 `radio-backend/static/`
- `build.emptyOutDir: true` — 每次构建清空旧产物
- `publicDir: 'public'` — `public/` 下的文件（sw.js, manifest.json）原样复制到输出

### 开发规范 / Conventions

- **TypeScript 优先** — 所有新增代码使用 `.ts` / `.vue` SFC
- **SFC `<script setup lang="ts">`** — 使用 Composition API + `<script setup>` 语法
- **响应式全局 store** — 新增状态到 `store.ts` 的 `reactive()` 对象
- **API 封装** — 新增接口调用到 `api.ts`
- **不硬编码端口** — 用 `window.location.origin` 计算
- **CSS 变量** — 所有颜色通过 CSS 自定义属性
- **组件拆分** — 按功能视图拆分，提取可复用共享组件；管理面板每个子标签独立组件
- **状态提示** — 使用 `<StatusMessage>` 组件，不重复内联样式
- **路由守卫** — 需要认证的管理页面通过 `router.beforeEach` 保护
