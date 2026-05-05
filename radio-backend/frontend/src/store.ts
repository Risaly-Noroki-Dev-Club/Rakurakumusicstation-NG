import { reactive } from 'vue'
import type {
  DeviceUser, Song, QueueItem, PlaybackState, LyricsLine,
  Toast, Playlist, AdminStats, ThemeName, NcmStatus
} from './types'

export const THEMES: ThemeName[] = ['auto', 'light', 'dark']

function getInitialThemeIndex(): number {
  const saved = localStorage.getItem('radio_theme') || 'auto'
  const idx = THEMES.indexOf(saved as ThemeName)
  return idx >= 0 ? idx : 0
}

export interface Store {
  deviceUser: DeviceUser | null
  stationName: string
  needsSetup: boolean
  themeIdx: number
  coverLoadError: boolean
  playbackState: PlaybackState
  lyricsLines: LyricsLine[]
  useFileMode: boolean
  queue: QueueItem[]
  history: QueueItem[]
  searchQuery: string
  searchResults: Song[]
  myPlaylists: Playlist[]
  newPlaylistName: string
  users: DeviceUser[]
  adminLogs: Array<{ created_at: string; action: string; details: string }>
  adminSongs: Song[]
  adminStats: AdminStats | null
  uploadFile: File | null
  uploadFileName: string
  uploadStatus: string
  uploadStatusType: string
  downloadPlaylist: string
  downloadQuality: string
  downloadFormat: string
  downloadRunning: boolean
  downloadStatusMsg: string
  downloadStatusType: string
  downloadLog: string
  ncmBadge: string
  ncmBadgeClass: string
  ncmActiveTab: 'cookie' | 'phone'
  ncmCookie: string
  ncmPhone: string
  ncmPassword: string
  ncmResult: string
  ncmResultType: string
  settingsStationName: string
  settingsSubtitle: string
  settingsPrimaryColor: string
  settingsSecondaryColor: string
  settingsBgColor: string
  settingsAdminPassword: string
  settingsResult: string
  settingsResultType: string
  userNcmBadge: string
  userNcmBadgeClass: string
  userNcmActiveTab: 'cookie' | 'phone'
  userNcmCookie: string
  userNcmPhone: string
  userNcmPassword: string
  userNcmResult: string
  userNcmResultType: string
  toasts: Toast[]
}

export const store: Store = reactive({
  deviceUser: null as DeviceUser | null,
  stationName: '电台',
  needsSetup: false,
  themeIdx: getInitialThemeIndex(),
  coverLoadError: false,
  playbackState: {
    song_id: 0, title: '', artist: '', position_ms: 0,
    duration_ms: 0, lyrics_line: null, status: 'stopped', cover_url: ''
  },
  lyricsLines: [] as LyricsLine[],
  useFileMode: false,
  queue: [] as QueueItem[],
  history: [] as QueueItem[],
  searchQuery: '',
  searchResults: [] as Song[],
  myPlaylists: [] as Playlist[],
  newPlaylistName: '',
  users: [] as DeviceUser[],
  adminLogs: [],
  adminSongs: [] as Song[],
  adminStats: null,
  uploadFile: null as File | null,
  uploadFileName: '',
  uploadStatus: '',
  uploadStatusType: '',
  downloadPlaylist: '',
  downloadQuality: 'exhigh',
  downloadFormat: 'mp3',
  downloadRunning: false,
  downloadStatusMsg: '',
  downloadStatusType: '',
  downloadLog: '',
  ncmBadge: '未配置',
  ncmBadgeClass: 'none',
  ncmActiveTab: 'cookie' as 'cookie' | 'phone',
  ncmCookie: '',
  ncmPhone: '',
  ncmPassword: '',
  ncmResult: '',
  ncmResultType: '',
  settingsStationName: '',
  settingsSubtitle: '',
  settingsPrimaryColor: '#764ba2',
  settingsSecondaryColor: '#667eea',
  settingsBgColor: '#f4f4f9',
  settingsAdminPassword: '',
  settingsResult: '',
  settingsResultType: '',
  userNcmBadge: '未配置',
  userNcmBadgeClass: 'none',
  userNcmActiveTab: 'cookie' as 'cookie' | 'phone',
  userNcmCookie: '',
  userNcmPhone: '',
  userNcmPassword: '',
  userNcmResult: '',
  userNcmResultType: '',
  toasts: [] as Toast[],
})

export function formatTime(ms: number | undefined | null): string {
  if (!ms || ms < 0) return '0:00'
  const secs = Math.floor(ms / 1000)
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return m + ':' + s.toString().padStart(2, '0')
}

export function toast(message: string, level: 'info' | 'success' | 'error' = 'info'): void {
  const id = Date.now() + Math.random()
  store.toasts.push({ id, message, level })
  setTimeout(() => {
    const idx = store.toasts.findIndex(t => t.id === id)
    if (idx >= 0) store.toasts.splice(idx, 1)
  }, 4000)
}

const ROOT = document.documentElement

export function applyTheme(): void {
  const theme = THEMES[store.themeIdx]
  if (theme === 'auto') {
    ROOT.removeAttribute('data-theme')
  } else {
    ROOT.setAttribute('data-theme', theme)
  }
  localStorage.setItem('radio_theme', theme)
}

export function cycleTheme(): void {
  store.themeIdx = (store.themeIdx + 1) % THEMES.length
  applyTheme()
}

applyTheme()

export function applyStationColors(info: {
  primary_color?: string
  secondary_color?: string
  bg_color?: string
}): void {
  if (info.primary_color)
    ROOT.style.setProperty('--primary', info.primary_color)
  if (info.secondary_color)
    ROOT.style.setProperty('--secondary', info.secondary_color)
  if (info.bg_color)
    ROOT.style.setProperty('--bg', info.bg_color)
}

export function parseLyrics(lrcText: string): void {
  const lines: LyricsLine[] = []
  const re = /\[(\d{1,3}):(\d{1,2})(?:\.(\d{1,3}))?\](.*)/g
  let match: RegExpExecArray | null
  while ((match = re.exec(lrcText)) !== null) {
    const min = parseInt(match[1])
    const sec = parseInt(match[2])
    const ms = match[3] ? parseInt(match[3].padEnd(3, '0')) : 0
    const timeMs = min * 60000 + sec * 1000 + ms
    const text = match[4].trim()
    if (text) lines.push({ timeMs, text })
  }
  lines.sort((a, b) => a.timeMs - b.timeMs)
  const merged: LyricsLine[] = []
  for (const l of lines) {
    if (merged.length > 0 && merged[merged.length - 1].timeMs === l.timeMs) {
      merged[merged.length - 1].text = l.text
    } else {
      merged.push(l)
    }
  }
  store.lyricsLines = merged
}
