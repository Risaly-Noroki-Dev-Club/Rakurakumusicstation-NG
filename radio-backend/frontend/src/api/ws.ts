// WebSocket client — connect once, reconnect with backoff, dispatch into the store.

import { wsUrl } from '@/api/client'
import { useStore } from '@/store'
import type { WsMessage } from '@/types'

const MAX_RECONNECT_ATTEMPTS = 20
const BASE_RECONNECT_DELAY_MS = 3000
const MAX_RECONNECT_DELAY_MS = 30000

let socket: WebSocket | null = null
let reconnectAttempts = 0
let reconnectTimer: number | null = null
let shouldReconnect = true

function scheduleReconnect() {
  if (!shouldReconnect || reconnectTimer !== null) return
  if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) return
  const delay = Math.min(BASE_RECONNECT_DELAY_MS * 1.5 ** reconnectAttempts, MAX_RECONNECT_DELAY_MS)
  reconnectAttempts += 1
  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null
    connectWebSocket()
  }, delay)
}

function handleMessage(raw: string) {
  let msg: WsMessage
  try {
    msg = JSON.parse(raw) as WsMessage
  } catch {
    return
  }
  const store = useStore.getState()
  switch (msg.type) {
    case 'playback_state':
      store.applyPlaybackState(msg)
      break
    case 'queue_update':
      store.applyQueueUpdate(msg)
      break
    case 'notice':
      store.addToast(msg.message, msg.level)
      break
    case 'ping':
      // Server requires a literal "pong" reply within 60s or it disconnects us.
      if (socket && socket.readyState === WebSocket.OPEN) socket.send('pong')
      break
    case 'listeners_update':
      store.applyListeners(msg)
      break
  }
}

export function connectWebSocket() {
  if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) return
  try {
    socket = new WebSocket(wsUrl('/ws'))
  } catch {
    scheduleReconnect()
    return
  }
  socket.onopen = () => {
    reconnectAttempts = 0
    useStore.getState().setWsConnected(true)
  }
  socket.onmessage = (event) => {
    if (typeof event.data === 'string') handleMessage(event.data)
  }
  socket.onclose = () => {
    useStore.getState().setWsConnected(false)
    socket = null
    scheduleReconnect()
  }
  socket.onerror = () => {
    // onclose follows; nothing to do here
  }
}

export function closeWebSocket() {
  shouldReconnect = false
  if (reconnectTimer !== null) {
    window.clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  if (socket) {
    socket.close()
    socket = null
  }
}

export function isWebSocketOpen(): boolean {
  return socket !== null && socket.readyState === WebSocket.OPEN
}

// ── polling fallback (used while WS is down) ───────────────

export function startPollers() {
  const pollPlayback = () => {
    if (isWebSocketOpen()) return
    void import('@/api').then(({ fetchNowPlaying }) =>
      fetchNowPlaying()
        .then((np) => {
          const store = useStore.getState()
          const prev = store.playback
          const changed = prev === null || prev.songId !== (np.song?.id ?? -1)
          store.applyPlaybackState({
            type: 'playback_state',
            song_id: np.song?.id ?? -1,
            title: np.song?.title ?? '',
            artist: np.song?.artist ?? '',
            position_ms: np.position_ms,
            duration_ms: np.duration_ms,
            lyrics_line: np.lyrics_line,
            lyrics_lines: changed && np.lyrics_text ? null : prev?.lyricsLines ?? null,
            status: np.song ? 'playing' : 'stopped',
            stream_url: np.stream_url,
            file_url: np.file_url,
            cover_url: np.cover_url,
            timestamp_ms: Date.now(),
          })
        })
        .catch(() => undefined),
    )
  }
  const pollQueue = () => {
    if (isWebSocketOpen()) return
    void import('@/api').then(({ fetchQueue }) => fetchQueue().then((q) => useStore.getState().setQueue(q)).catch(() => undefined))
  }
  pollPlayback()
  pollQueue()
  const playbackTimer = window.setInterval(pollPlayback, 2000)
  const queueTimer = window.setInterval(pollQueue, 5000)
  return () => {
    window.clearInterval(playbackTimer)
    window.clearInterval(queueTimer)
  }
}
