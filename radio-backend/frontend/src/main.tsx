import React from 'react'
import ReactDOM from 'react-dom/client'
import { RouterProvider } from 'react-router-dom'
import { ThemeProvider } from '@appica/ui-react/providers/theme-provider'
import { registerSW } from 'virtual:pwa-register'
import { router } from './router'
import './index.css'

// Service Worker（vite-plugin-pwa 生成）：预缓存静态产物，支撑 PWA 安装
// 与离线壳。autoUpdate：后台有新版本时自动接管并刷新。
registerSW({ immediate: true })

const rootEl = document.getElementById('root')
if (!rootEl) throw new Error('#root not found')

ReactDOM.createRoot(rootEl).render(
  <React.StrictMode>
    <ThemeProvider>
      <RouterProvider router={router} />
    </ThemeProvider>
  </React.StrictMode>,
)
