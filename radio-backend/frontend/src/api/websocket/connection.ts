import { appPath } from '../client'
import { toast } from '../../store'
import { handleWsMessage } from './messages'
import type { WsMessage } from '../../types'

let ws: WebSocket | null = null
let wsReconnectAttempts = 0
let wsReconnectTimer: ReturnType<typeof setTimeout> | null = null
let shouldReconnect = true
const WS_MAX_RECONNECT_ATTEMPTS = 20
const WS_BASE_RECONNECT_DELAY = 3000

function getWsUrl(): string {
  const proto = window.location.protocol === 'https:' ? 'wss://' : 'ws://'
  return proto + window.location.host + appPath('/ws')
}

export function connectWebSocket(): void {
  shouldReconnect = true
  if (ws?.readyState === WebSocket.CONNECTING || ws?.readyState === WebSocket.OPEN) return
  if (wsReconnectTimer) {
    clearTimeout(wsReconnectTimer)
    wsReconnectTimer = null
  }

  try {
    const socket = new WebSocket(getWsUrl())
    ws = socket
    socket.onopen = () => {
      wsReconnectAttempts = 0
      console.log('[WS] Connected')
      toast('已连接到电台服务器', 'success')
    }
    socket.onmessage = (event) => {
      try {
        const msg: WsMessage = JSON.parse(event.data)
        handleWsMessage(msg, socket)
      } catch { /* ignore */ }
    }
    socket.onclose = () => {
      if (ws !== socket) return
      ws = null
      if (!shouldReconnect) return
      wsReconnectAttempts++
      if (wsReconnectAttempts > WS_MAX_RECONNECT_ATTEMPTS) {
        console.log('[WS] Max reconnection attempts reached')
        toast('WebSocket 连接失败，请刷新页面', 'error')
        return
      }
      const delay = Math.min(WS_BASE_RECONNECT_DELAY * Math.pow(1.5, wsReconnectAttempts - 1), 30000)
      console.log(`[WS] Disconnected, reconnecting in ${delay}ms (attempt ${wsReconnectAttempts})...`)
      wsReconnectTimer = setTimeout(() => {
        wsReconnectTimer = null
        connectWebSocket()
      }, delay)
    }
    socket.onerror = () => { /* ignore */ }
  } catch {
    if (shouldReconnect) wsReconnectTimer = setTimeout(connectWebSocket, 3000)
  }
}

export function getWs(): WebSocket | null { return ws }

export function closeWebSocket(): void {
  shouldReconnect = false
  if (wsReconnectTimer) {
    clearTimeout(wsReconnectTimer)
    wsReconnectTimer = null
  }
  if (ws) {
    ws.close()
    ws = null
  }
}

export function isWebSocketOpen(): boolean {
  return !!ws && ws.readyState === WebSocket.OPEN
}
