import { create } from 'zustand'
import type {
  AuthUser,
  HistoryItem,
  ListenersUpdateWs,
  LyricsLine,
  PlaybackStateWs,
  PlaybackStatus,
  QueueItemDisplay,
  QueueUpdateWs,
  StationInfo,
} from '@/types'

export interface Toast {
  id: number
  message: string
  level: 'info' | 'success' | 'warning' | 'error'
}

export interface Playback {
  songId: number
  title: string
  artist: string
  positionMs: number
  durationMs: number
  lyricsLine: number | null
  /** Full lyrics for the current song; null until the song-change frame arrives. */
  lyricsLines: LyricsLine[] | null
  /** [] means "no lyrics for this song" — distinct from null ("not resent"). */
  lyricsKnown: boolean
  status: PlaybackStatus
  streamUrl: string
  fileUrl: string | null
  coverUrl: string | null
  timestampMs: number
}

interface AppStore {
  station: StationInfo | null
  auth: AuthUser | null
  playback: Playback | null
  queue: QueueItemDisplay[]
  history: HistoryItem[]
  listeners: { count: number; names: string[] }
  toasts: Toast[]
  wsConnected: boolean
  audioPaused: boolean
  needsPlay: boolean
  accent: string

  setStation: (station: StationInfo | null) => void
  setAuth: (auth: AuthUser | null) => void
  setQueue: (queue: QueueItemDisplay[]) => void
  setHistory: (history: HistoryItem[]) => void
  setWsConnected: (connected: boolean) => void
  setAudioPaused: (paused: boolean) => void
  setNeedsPlay: (needs: boolean) => void
  setAccent: (accent: string) => void
  addToast: (message: string, level?: Toast['level']) => void
  removeToast: (id: number) => void
  applyPlaybackState: (msg: PlaybackStateWs) => void
  applyQueueUpdate: (msg: QueueUpdateWs) => void
  applyListeners: (msg: ListenersUpdateWs) => void
}

let toastSeq = 0

function songKey(p: Playback | null): string {
  return p ? `${p.songId}|${p.title}|${p.streamUrl}` : ''
}

export const useStore = create<AppStore>((set, get) => ({
  station: null,
  auth: null,
  playback: null,
  queue: [],
  history: [],
  listeners: { count: 0, names: [] },
  toasts: [],
  wsConnected: false,
  audioPaused: false,
  needsPlay: false,
  accent: localStorage.getItem('rakuraku.accent') ?? '#764ba2',

  setStation: (station) => set({ station }),
  setAuth: (auth) => set({ auth }),
  setQueue: (queue) => set({ queue }),
  setHistory: (history) => set({ history }),
  setWsConnected: (wsConnected) => set({ wsConnected }),
  setAudioPaused: (audioPaused) => set({ audioPaused }),
  setNeedsPlay: (needsPlay) => set({ needsPlay }),
  setAccent: (accent) => {
    localStorage.setItem('rakuraku.accent', accent)
    set({ accent })
  },

  addToast: (message, level = 'info') => {
    const id = ++toastSeq
    set((s) => ({ toasts: [...s.toasts, { id, message, level }] }))
    window.setTimeout(() => get().removeToast(id), 4000)
  },
  removeToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

  applyPlaybackState: (msg) => {
    const prev = get().playback
    const changedSong = songKey(prev) !== `${msg.song_id}|${msg.title}|${msg.stream_url}`
    const lyricsLines = changedSong && msg.lyrics_lines !== null ? msg.lyrics_lines : prev?.lyricsLines ?? null
    const lyricsKnown = changedSong ? true : (prev?.lyricsKnown ?? false)

    set({
      playback: {
        songId: msg.song_id,
        title: msg.title,
        artist: msg.artist,
        positionMs: msg.position_ms,
        durationMs: msg.duration_ms,
        lyricsLine: msg.lyrics_line,
        lyricsLines,
        lyricsKnown,
        status: msg.status,
        streamUrl: msg.stream_url,
        fileUrl: msg.file_url,
        coverUrl: msg.cover_url,
        timestampMs: msg.timestamp_ms,
      },
    })

    // Browser notification on song change when the tab is hidden.
    if (changedSong && document.hidden && typeof Notification !== 'undefined' && Notification.permission === 'granted') {
      try {
        new Notification(msg.title, { body: msg.artist })
      } catch {
        // some browsers reject construction without a service worker
      }
    }
  },

  applyQueueUpdate: (msg) => {
    const { queue } = get()
    if (msg.action === 'added' && msg.song_title) {
      get().addToast(`已点播：${msg.song_title}`, 'info')
      void import('@/api').then(({ fetchQueue }) => fetchQueue().then((q) => get().setQueue(q)).catch(() => undefined))
    }
    if (queue.length !== msg.queue_size) {
      void import('@/api').then(({ fetchQueue }) => fetchQueue().then((q) => get().setQueue(q)).catch(() => undefined))
    }
  },

  applyListeners: (msg) => set({ listeners: { count: msg.count, names: msg.names } }),
}))
