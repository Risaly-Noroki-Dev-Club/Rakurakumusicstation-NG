import { appPath } from '../client'
import { store, toast } from '../../store'
import { handleWsMessage } from './messages'
import type { WsMessage } from '../../types'

let ws: WebSocket | null = null
let wsReconnectAttempts = 0
const WS_MAX_RECONNECT_ATTEMPTS = 20
const WS_BASE_RECONNECT_DELAY = 3000

function getWsUrl(): string {
  const proto = window.location.protocol === 'https:' ? 'wss://' : 'ws://'
  let url = proto + window.location.host + appPath('/ws')
  if (store.deviceUser?.device_token) {
    url += '?device_token=' + encodeURIComponent(store.deviceUser.device_token)
  }
  return url
}

export function connectWebSocket(): void {
  try {
    ws = new WebSocket(getWsUrl())
    ws.onopen = () => {
      wsReconnectAttempts = 0
      console.log('[WS] Connected')
      toast('已连接到电台服务器', 'success')
    }
    ws.onmessage = (event) => {
      try {
        const msg: WsMessage = JSON.parse(event.data)
        handleWsMessage(msg, ws)
      } catch { /* ignore */ }
    }
    ws.onclose = () => {
      wsReconnectAttempts++
      if (wsReconnectAttempts > WS_MAX_RECONNECT_ATTEMPTS) {
        console.log('[WS] Max reconnection attempts reached')
        toast('WebSocket 连接失败，请刷新页面', 'error')
        return
      }
      const delay = Math.min(WS_BASE_RECONNECT_DELAY * Math.pow(1.5, wsReconnectAttempts - 1), 30000)
      console.log(`[WS] Disconnected, reconnecting in ${delay}ms (attempt ${wsReconnectAttempts})...`)
      setTimeout(connectWebSocket, delay)
    }
    ws.onerror = () => { /* ignore */ }
  } catch {
    setTimeout(connectWebSocket, 3000)
  }
}

export function getWs(): WebSocket | null { return ws }

export function closeWebSocket(): void {
  if (ws) ws.close()
}

export function isWebSocketOpen(): boolean {
  return !!ws && ws.readyState === WebSocket.OPEN
}
