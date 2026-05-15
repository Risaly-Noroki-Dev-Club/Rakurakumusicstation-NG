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
  displayPositionMs: number
  lyricsLines: LyricsLine[]
  useFileMode: boolean
  queue: QueueItem[]
  history: QueueItem[]
  searchQuery: string
  searchResults: Song[]
  searchTotal: number
  searchOffset: number
  searchLoading: boolean
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

  // ─── New states for Apple-Music-style player ───
  showLyrics: boolean
  extractedColor: string
  isDesktop: boolean
  showSnackbar: boolean
  snackbarText: string
  snackbarColor: string
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
  displayPositionMs: 0,
  lyricsLines: [] as LyricsLine[],
  useFileMode: false,
  queue: [] as QueueItem[],
  history: [] as QueueItem[],
  searchQuery: '',
  searchResults: [] as Song[],
  searchTotal: 0,
  searchOffset: 0,
  searchLoading: false,
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
  settingsPrimaryColor: '#003D99',
  settingsSecondaryColor: '#00897B',
  settingsBgColor: '#FAFAFA',
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

  showLyrics: false,
  extractedColor: '#003D99',
  isDesktop: window.innerWidth >= 960,
  showSnackbar: false,
  snackbarText: '',
  snackbarColor: 'info',
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
  store.snackbarText = message
  store.snackbarColor = level
  store.showSnackbar = true
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
    ROOT.style.setProperty('--am-primary', info.primary_color)
  if (info.secondary_color)
    ROOT.style.setProperty('--am-secondary', info.secondary_color)
  if (info.bg_color)
    ROOT.style.setProperty('--am-bg', info.bg_color)
}

// Responsive listener
window.addEventListener('resize', () => {
  store.isDesktop = window.innerWidth >= 960
})
