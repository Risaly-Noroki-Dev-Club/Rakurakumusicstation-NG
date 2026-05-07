import { apiBase, getStreamUrl, getAudioEngineUrl } from './client'
import { store, toast } from '../store'
import { refreshQueue } from './queue'
import { onSearchInput } from './songs'
import type { WsMessage } from '../types'

let playbackPoller: ReturnType<typeof setInterval> | null = null
let ws: WebSocket | null = null
let wsReconnectAttempts = 0
const WS_MAX_RECONNECT_ATTEMPTS = 20
const WS_BASE_RECONNECT_DELAY = 3000

function getWsUrl(): string {
  const proto = window.location.protocol === 'https:' ? 'wss://' : 'ws://'
  let url = proto + window.location.host + '/ws'
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
        handleWsMessage(msg)
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

function handleWsMessage(msg: WsMessage): void {
  switch (msg.type) {
  case 'playback_state': {
    const prevSongId = store.playbackState.song_id
    store.playbackState.song_id = msg.song_id || 0
    store.playbackState.title = msg.title || ''
    store.playbackState.artist = msg.artist || ''
    store.playbackState.position_ms = msg.position_ms || 0
    store.playbackState.duration_ms = msg.duration_ms || 0
    store.playbackState.lyrics_line = msg.lyrics_line
    store.playbackState.status = msg.status || 'stopped'
    store.playbackState.cover_url = msg.cover_url || ''
    if (msg.song_id !== prevSongId) {
      store.coverLoadError = false
    }
    if (msg.lyrics_lines && (msg.song_id !== prevSongId || store.lyricsLines.length === 0)) {
      store.lyricsLines = msg.lyrics_lines
    }
    break
  }
  case 'queue_update':
    toast((msg.requested_by || '某人') + ' 为电台点了《' + (msg.song_title || '未知歌曲') + '》', 'info')
    refreshQueue()
    break
  case 'notice':
    toast(msg.message, msg.level === 'error' ? 'error' : 'info')
    break
  case 'ping':
    if (ws && ws.readyState === WebSocket.OPEN) ws.send('pong')
    break
  }
}

export function refreshPlaybackPoll(): void {
  if (ws && ws.readyState === WebSocket.OPEN) return
  fetch(apiBase + '/api/now-playing')
    .then(r => r.json())
    .then(resp => {
      if (!resp.success || !resp.data || !resp.data.song) return
      const d = resp.data
      const prevId = store.playbackState.song_id
      store.playbackState.song_id = d.song.id
      store.playbackState.title = d.song.title
      store.playbackState.artist = d.song.artist
      store.playbackState.position_ms = d.position_ms || 0
      store.playbackState.duration_ms = d.duration_ms || 0
      store.lyricsLines = []
      store.playbackState.lyrics_line = d.lyrics_line
      store.playbackState.cover_url = ''
      if (d.song.id !== prevId) store.coverLoadError = false
    }).catch(() => {})
}

export function startPollers(): { queuePoller: ReturnType<typeof setInterval>, playbackPoller: ReturnType<typeof setInterval> } {
  playbackPoller = setInterval(refreshPlaybackPoll, 2000)
  const queuePoller = setInterval(refreshQueue, 5000)
  return { queuePoller, playbackPoller }
}

export function stopPollers(qp: ReturnType<typeof setInterval>): void {
  if (playbackPoller) clearInterval(playbackPoller)
  if (qp) clearInterval(qp)
  if (ws) ws.close()
}

export function getWs(): WebSocket | null { return ws }

// ─── Search debounce ────────────────────────────────────

let searchTimer: ReturnType<typeof setTimeout> | null = null
export function debouncedSearch(): void {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(onSearchInput, 300)
}

// ─── Playback mode switching ────────────────────────────

let mediaSource: MediaSource | null = null
let sourceBuffer: SourceBuffer | null = null

export function switchPlaybackMode(audioEl: HTMLAudioElement): void {
  store.useFileMode = !store.useFileMode
  if (store.useFileMode) {
    if (typeof MediaSource !== 'undefined') {
      startFilePlayback(audioEl)
    } else {
      toast('你的浏览器不支持推文件模式，使用推流模式', 'error')
      store.useFileMode = false
    }
  } else {
    audioEl.src = getStreamUrl()
    audioEl.load()
    audioEl.play().catch(() => {})
  }
}

function startFilePlayback(audio: HTMLAudioElement): void {
  mediaSource = new MediaSource()
  audio.src = URL.createObjectURL(mediaSource)
  mediaSource.addEventListener('sourceopen', async () => {
    try {
      sourceBuffer = mediaSource!.addSourceBuffer('audio/mpeg')
      fetchFileChunk(0)
    } catch {
      toast('推文件模式初始化失败，回退到推流模式', 'error')
      store.useFileMode = false
      audio.src = getStreamUrl()
      audio.load()
      audio.play().catch(() => {})
    }
  })
}

async function fetchFileChunk(offset: number): Promise<void> {
  if (store.playbackState.song_id <= 0) return
  try {
    const res = await fetch(getAudioEngineUrl() + '/file/' + store.playbackState.song_id, {
      headers: { 'Range': 'bytes=' + offset + '-' }
    })
    if (!res.ok) return
    const buffer = await res.arrayBuffer()
    if (buffer.byteLength > 0 && sourceBuffer && !sourceBuffer.updating) {
      sourceBuffer.appendBuffer(buffer)
    }
  } catch { /* ignore */ }
}

// ─── Volume helpers ─────────────────────────────────────

export function volumeDown(audioEl: HTMLAudioElement): void {
  audioEl.volume = Math.max(0, audioEl.volume - 0.1)
}
export function volumeUp(audioEl: HTMLAudioElement): void {
  audioEl.volume = Math.min(1, audioEl.volume + 0.1)
}
