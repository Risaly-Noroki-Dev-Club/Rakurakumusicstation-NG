import { apiFetch } from '../client'
import { store } from '../../store'
import { isWebSocketOpen } from './connection'
import {
  currentAnchorPosition,
  startInterpolation,
  updateAnchor
} from './interpolation'

export function refreshPlaybackPoll(): void {
  if (isWebSocketOpen()) return
  apiFetch('/api/now-playing')
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
      updateAnchor(d.position_ms || 0)
      store.displayPositionMs = currentAnchorPosition()
      startInterpolation()
    }).catch(() => {})
}
