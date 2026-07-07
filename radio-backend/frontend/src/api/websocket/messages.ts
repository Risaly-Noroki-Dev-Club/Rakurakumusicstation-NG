import { store, toast } from '../../store'
import { refreshQueue } from '../queue'
import {
  currentAnchorPosition,
  startInterpolation,
  updateAnchor
} from './interpolation'
import type { WsMessage } from '../../types'

function sendBrowserNotification(title: string, body: string): void {
  if (!('Notification' in window)) return
  if (Notification.permission !== 'granted') return
  if (document.visibilityState === 'visible') return

  try {
    new Notification(title, {
      body,
      icon: '/icon-192.png',
      tag: 'rakuraku-notification',
    })
  } catch {
    // Notification constructor may fail in some contexts
  }
}

export function handleWsMessage(msg: WsMessage, ws: WebSocket | null): void {
  switch (msg.type) {
  case 'playback_state': {
    const prevSongId = store.playbackState.song_id
    const newSongId = msg.song_id || 0

    // Send notification on song change when page is in background
    if (newSongId !== prevSongId && newSongId > 0 && msg.title) {
      sendBrowserNotification(
        '正在播放',
        msg.artist ? `${msg.title} - ${msg.artist}` : msg.title
      )
    }

    store.playbackState.song_id = newSongId
    store.playbackState.title = msg.title || ''
    store.playbackState.artist = msg.artist || ''
    store.playbackState.position_ms = msg.position_ms || 0
    store.playbackState.duration_ms = msg.duration_ms || 0
    store.playbackState.lyrics_line = msg.lyrics_line
    store.playbackState.status = msg.status || 'stopped'
    store.playbackState.cover_url = msg.cover_url || ''

    updateAnchor(msg.position_ms || 0)

    if (newSongId !== prevSongId) {
      store.displayPositionMs = currentAnchorPosition()
      store.coverLoadError = false
    }

    if (newSongId !== prevSongId || store.lyricsLines.length === 0) {
      if (msg.lyrics_lines !== null && msg.lyrics_lines !== undefined) {
        store.lyricsLines = msg.lyrics_lines
      } else if (newSongId !== prevSongId) {
        store.lyricsLines = []
      }
    }

    startInterpolation()
    break
  }
  case 'queue_update':
    toast('有人为电台点了《' + (msg.song_title || '未知歌曲') + '》', 'info')
    sendBrowserNotification('新歌点播', `有人点了《${msg.song_title || '未知歌曲'}》`)
    refreshQueue()
    break
  case 'notice':
    toast(msg.message, msg.level === 'error' ? 'error' : 'info')
    if (msg.level === 'error') {
      sendBrowserNotification('电台通知', msg.message)
    }
    break
  case 'ping':
    if (ws && ws.readyState === WebSocket.OPEN) ws.send('pong')
    break
  case 'listeners_update':
    store.onlineListenerCount = msg.count || 0
    store.onlineListenerNames = msg.names || []
    break
  }
}
