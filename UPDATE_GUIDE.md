# 更新指南：切换到 Vue 单页面前端

本文说明本次更新后的构建、部署和访问方式变化。

## 主要变化

- 页面不再由后端 Askama 模板渲染，统一改为 Vue SPA。
- 后端仍是单二进制 `radio-backend`，继续服务 REST API、WebSocket `/ws`、音频流 `/stream` 和静态前端文件。
- `/`、`/library`、`/up-next`、`/settings`、`/admin/*` 等页面路径都会返回 `static/index.html`，由前端路由接管。
- 旧的 `/queue` 入口会兼容跳转到 `/up-next`。
- `/api/*` 路由仍返回 JSON；不存在的 API 不会回退到 HTML 页面。

## 构建顺序

前端改动后，必须先构建前端，再打包发布目录：

```bash
cd radio-backend/frontend
npm run build
cd ../..
./build_release.sh
```

注意：`./build_release.sh` 只复制现有的 `radio-backend/static/` 到 `dist/static/`，不会自动运行 Vite。

## 后端单独构建

只验证 Rust 后端时可以运行：

```bash
cd radio-backend
cargo build
```

## 开发模式

开发前端时运行 Vite：

```bash
cd radio-backend/frontend
npm run dev
```

Vite 监听 `5173`，并把 `/api`、`/ws`、`/stream` 代理到 `localhost:2241`。

## 部署后检查

发布后从 `dist/` 启动：

```bash
cd dist
./start.sh
```

建议检查：

- 清空浏览器 Cookie 后访问 `/`，应直接进入 Vue 页面，不应显示 `{"error":"Not authenticated","success":false}`。
- 直接访问 `/library`、`/up-next`、`/settings`、`/admin/users`，应由 SPA 正常打开。
- 访问 `/queue` 应跳转到 `/up-next`。
- 访问 `/api/station` 应返回 JSON。
- 访问不存在的 `/api/...` 应返回 JSON 404，而不是前端 HTML。

## 首次认证变化

后端会在首次无 `device_token` 的请求中创建设备用户，并在同一次请求内让受保护 API 能读取新 token，同时通过 `Set-Cookie` 写入浏览器。

这避免首次进入电台时先看到未认证 JSON，再刷新才正常的问题。
