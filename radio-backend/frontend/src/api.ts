import { store, parseLyrics, applyStationColors, toast } from './store'
import type { WsMessage } from './types'

const apiBase = window.location.origin

let streamUrl = '/stream'
export let audioEngineUrl = ''

export function getStreamUrl(): string { return streamUrl }
export function getAudioEngineUrl(): string { return audioEngineUrl }
export function getBackendUrl(): string { return apiBase }

// ─── Station info ───────────────────────────────────────

export async function loadStationInfo(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/station')
    const data = await res.json()
    if (data) {
      const info = data.data || data
      store.stationName = info.name
      store.needsSetup = info.needs_setup === true
      document.title = info.name
      if (info.primary_color) {
        const metaTheme = document.querySelector('meta[name="theme-color"]') as HTMLMetaElement | null
        if (metaTheme) metaTheme.content = info.primary_color
        document.documentElement.style.setProperty('--primary', info.primary_color)
      }
      applyStationColors(info)
      if (info.stream_url) {
        streamUrl = info.stream_url
        try {
          const u = new URL(info.stream_url, window.location.origin)
          audioEngineUrl = u.origin
        } catch { /* ignore */ }
      }
    }
  } catch { /* ignore */ }
}

// ─── Device identity ─────────────────────────────────────

export async function loadDeviceUser(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/auth/me')
    const data = await res.json()
    if (data.success && data.data) {
      store.deviceUser = {
        id: data.data.id,
        display_name: data.data.display_name || ('Listener-' + String(data.data.id).padStart(4, '0')),
        role: data.data.role,
        device_token: data.data.device_token || ''
      }
      loadMyPlaylists()
      loadUserNcmStatus()
    }
  } catch { /* ignore */ }
}

export async function setDisplayName(name: string): Promise<boolean> {
  try {
    const res = await fetch(apiBase + '/api/auth/name', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ display_name: name })
    })
    const data = await res.json()
    if (data.success) {
      if (store.deviceUser) store.deviceUser.display_name = name
      toast('显示名称已更新', 'success')
      return true
    } else {
      toast(data.error || '更新失败', 'error')
      return false
    }
  } catch {
    toast('请求失败', 'error')
    return false
  }
}

export async function claimAdmin(token: string): Promise<boolean> {
  try {
    const res = await fetch(apiBase + '/api/auth/claim-admin', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ admin_setup_token: token })
    })
    const data = await res.json()
    if (data.success) {
      if (store.deviceUser) store.deviceUser.role = 'admin'
      toast('已获得管理员权限', 'success')
      return true
    } else {
      toast(data.error || '验证失败', 'error')
      return false
    }
  } catch {
    toast('请求失败', 'error')
    return false
  }
}

// ─── Queue ──────────────────────────────────────────────

export async function refreshQueue(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/queue')
    const data = await res.json()
    if (!data.success) return
    store.queue = data.data || []
    await refreshHistory()
  } catch { /* ignore */ }
}

export async function refreshHistory(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/queue/history')
    const data = await res.json()
    if (data.success) store.history = (data.data || []).slice(0, 20)
  } catch { /* ignore */ }
}

export async function removeQueueItem(id: number): Promise<void> {
  try {
    await fetch(apiBase + '/api/queue/' + id, { method: 'DELETE' })
    refreshQueue()
  } catch { toast('移除失败', 'error') }
}

export async function onSearchInput(): Promise<void> {
  const q = store.searchQuery.trim()
  try {
    const res = await fetch(apiBase + '/api/songs?q=' + encodeURIComponent(q) + '&limit=50')
    const data = await res.json()
    if (data.success) {
      store.searchResults = data.data && data.data.data ? data.data.data : []
    }
  } catch { /* ignore */ }
}

export async function addToQueue(songId: number): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/queue', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ song_id: songId })
    })
    const data = await res.json()
    if (data.success) {
      toast('已添加到电台队列！', 'success')
      refreshQueue()
    } else {
      toast(data.error || '点歌失败', 'error')
    }
  } catch { toast('请求失败', 'error') }
}

export async function downloadSong(songId: number): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/songs/' + songId + '/download')
    if (!res.ok) {
      const data = await res.json().catch(() => ({ error: '下载失败' }))
      toast(data.error || '下载失败', 'error')
      return
    }
    const blob = await res.blob()
    const disposition = res.headers.get('Content-Disposition') || ''
    const match = disposition.match(/filename="(.+)"/)
    const filename = match ? match[1] : 'song_' + songId + '.mp3'
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    toast('下载完成', 'success')
  } catch { toast('下载失败', 'error') }
}

export async function uploadSong(file: File): Promise<boolean> {
  if (file.size > 100 * 1024 * 1024) {
    toast('文件大小超过 100MB 限制', 'error')
    return false
  }
  const formData = new FormData()
  formData.append('file', file)
  try {
    const res = await fetch(apiBase + '/api/songs/upload', {
      method: 'POST',
      body: formData
    })
    const data = await res.json()
    if (data.success) {
      toast(data.data || '上传成功', 'success')
      return true
    } else {
      toast(data.error || '上传失败', 'error')
      return false
    }
  } catch {
    toast('上传失败', 'error')
    return false
  }
}

// ─── NCM ────────────────────────────────────────────────

export async function loadUserNcmStatus(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/ncm')
    if (!res.ok) return
    const d = await res.json()
    if (!d.success) return
    const data = d.data
    if (data.configured) {
      const label = data.method === 'cookie' ? 'Cookie 已配置' : '手机号 ' + (data.phone_hint || '') + ' 已配置'
      store.userNcmBadge = '✓ ' + label
      store.userNcmBadgeClass = 'ok'
    } else {
      store.userNcmBadge = '未配置（游客模式）'
      store.userNcmBadgeClass = 'none'
    }
  } catch { /* ignore */ }
}

export async function saveUserNcmSettings(): Promise<void> {
  const payload = store.userNcmActiveTab === 'cookie'
    ? { cookie: store.userNcmCookie.trim(), phone: '', password: '' }
    : { phone: store.userNcmPhone.trim(), password: store.userNcmPassword, cookie: '' }
  if (store.userNcmActiveTab === 'cookie' && !payload.cookie) {
    store.userNcmResult = '请填写 Cookie'
    store.userNcmResultType = 'error'
    return
  }
  if (store.userNcmActiveTab === 'phone' && (!payload.phone || !payload.password)) {
    store.userNcmResult = '请填写手机号和密码'
    store.userNcmResultType = 'error'
    return
  }
  try {
    const res = await fetch(apiBase + '/api/ncm', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    const data = await res.json()
    if (data.success) {
      store.userNcmResult = '✅ 保存成功'
      store.userNcmResultType = 'success'
      loadUserNcmStatus()
    } else {
      store.userNcmResult = '❌ ' + (data.error || '保存失败')
      store.userNcmResultType = 'error'
    }
  } catch {
    store.userNcmResult = '❌ 请求失败'
    store.userNcmResultType = 'error'
  }
}

export async function testUserNcmLogin(): Promise<void> {
  store.userNcmResult = '测试中...'
  store.userNcmResultType = 'info'
  try {
    const res = await fetch(apiBase + '/api/ncm/test', {
      method: 'POST'
    })
    const data = await res.json()
    if (data.success) {
      const d = data.data
      store.userNcmResult = (d.success ? '✅ ' : '❌ ') + (d.output || (d.success ? '登录成功' : '登录失败'))
      store.userNcmResultType = d.success ? 'success' : 'error'
    } else {
      store.userNcmResult = '❌ 请求失败'
      store.userNcmResultType = 'error'
    }
  } catch {
    store.userNcmResult = '❌ 请求失败'
    store.userNcmResultType = 'error'
  }
}

// ─── Playlists ──────────────────────────────────────────

export async function loadMyPlaylists(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/playlists')
    const data = await res.json()
    if (data.success) store.myPlaylists = data.data || []
  } catch { /* ignore */ }
}

export async function createPlaylist(): Promise<void> {
  if (!store.newPlaylistName.trim()) return
  try {
    const res = await fetch(apiBase + '/api/playlists', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: store.newPlaylistName.trim() })
    })
    const data = await res.json()
    if (data && data.success) {
      toast('歌单创建成功', 'success')
      store.newPlaylistName = ''
      await loadMyPlaylists()
    } else {
      toast((data && data.error) || '创建失败', 'error')
    }
  } catch { toast('请求失败', 'error') }
}

// ─── Search ──────────────────────────────────────────────

let searchTimer: ReturnType<typeof setTimeout> | null = null
export function debouncedSearch(): void {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(onSearchInput, 300)
}

// ─── Playback mode switching ────────────────────────────

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
    audioEl.src = streamUrl
    audioEl.load()
    audioEl.play().catch(() => {})
  }
}

let mediaSource: MediaSource | null = null
let sourceBuffer: SourceBuffer | null = null

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
      audio.src = streamUrl
      audio.load()
      audio.play().catch(() => {})
    }
  })
}

async function fetchFileChunk(offset: number): Promise<void> {
  if (store.playbackState.song_id <= 0) return
  try {
    const res = await fetch(audioEngineUrl + '/file/' + store.playbackState.song_id, {
      headers: { 'Range': 'bytes=' + offset + '-' }
    })
    if (!res.ok) return
    const buffer = await res.arrayBuffer()
    if (buffer.byteLength > 0 && sourceBuffer && !sourceBuffer.updating) {
      sourceBuffer.appendBuffer(buffer)
    }
  } catch { /* ignore */ }
}

// ─── WebSocket ──────────────────────────────────────────

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
    if (msg.lyrics_text && (msg.song_id !== prevSongId || store.lyricsLines.length === 0)) {
      parseLyrics(msg.lyrics_text)
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
      store.playbackState.lyrics_line = d.lyrics_line
      store.playbackState.cover_url = ''
      if (d.song.id !== prevId) store.coverLoadError = false
      if (d.lyrics_text) parseLyrics(d.lyrics_text)
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

export function volumeDown(audioEl: HTMLAudioElement): void {
  audioEl.volume = Math.max(0, audioEl.volume - 0.1)
}
export function volumeUp(audioEl: HTMLAudioElement): void {
  audioEl.volume = Math.min(1, audioEl.volume + 0.1)
}
