import { store, toast } from '../../store'
import { refreshQueue } from '../queue'
import {
  currentAnchorPosition,
  startInterpolation,
  updateAnchor
} from './interpolation'
import type { WsMessage } from '../../types'

export function handleWsMessage(msg: WsMessage, ws: WebSocket | null): void {
  switch (msg.type) {
  case 'playback_state': {
    const prevSongId = store.playbackState.song_id
    const newSongId = msg.song_id || 0

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

    if (msg.lyrics_lines && (newSongId !== prevSongId || store.lyricsLines.length === 0)) {
      store.lyricsLines = msg.lyrics_lines
    }

    startInterpolation()
    break
  }
  case 'queue_update':
    toast('有人为电台点了《' + (msg.song_title || '未知歌曲') + '》', 'info')
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
