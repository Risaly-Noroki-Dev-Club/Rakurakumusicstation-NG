export interface DeviceUser {
  id: number
  display_name: string
  role: string
  device_token: string
  created_at?: string
  banned_until?: string | null
}

export interface Song {
  id: number
  title: string
  artist: string
  album?: string
  duration_ms?: number
  cover_url?: string
}

export interface QueueItem {
  id: number
  song_id: number
  song?: Song
  requested_by: string
  status: string
  played_at?: string
  position?: number
}

export interface PlaybackState {
  song_id: number
  title: string
  artist: string
  position_ms: number
  duration_ms: number
  lyrics_line: number | null
  status: string
  cover_url: string
}

export interface LyricsLine {
  timeMs: number
  text: string
}

export interface Toast {
  id: number
  message: string
  level: 'info' | 'success' | 'error'
}

export interface Playlist {
  id: number
  name: string
  song_count?: number
  created_at?: string
}

export interface ApiResponse<T = unknown> {
  success: boolean
  data?: T
  error?: string
}

export interface StationInfo {
  name: string
  subtitle?: string
  primary_color?: string
  secondary_color?: string
  bg_color?: string
  stream_url?: string
  ws_url?: string
  needs_setup: boolean
}

export interface NowPlayingData {
  song: Song
  position_ms: number
  duration_ms: number
  lyrics_line: number | null
  lyrics_text?: string
  status?: string
}

export interface WsPlaybackState {
  type: 'playback_state'
  song_id: number
  title: string
  artist: string
  position_ms: number
  duration_ms: number
  lyrics_line: number | null
  lyrics_lines?: LyricsLine[]
  status: string
  cover_url: string
  stream_url?: string
  timestamp_ms?: number
}

export interface WsQueueUpdate {
  type: 'queue_update'
  action: string
  song_title: string
  requested_by: string
  queue_size: number
}

export interface WsNotice {
  type: 'notice'
  message: string
  level: string
}

export interface WsPing {
  type: 'ping'
  timestamp: number
}

export type WsMessage = WsPlaybackState | WsQueueUpdate | WsNotice | WsPing

export interface NcmStatus {
  configured: boolean
  method: string
  phone_hint?: string
}

export interface NcmResult {
  success: boolean
  output: string
}

export interface DownloadStatus {
  running: boolean
  log?: string
}

export interface AdminStats {
  users: number
  songs: number
  queue_size: number
  playlists: number
}

export interface SettingsData {
  station_name: string
  subtitle: string
  primary_color: string
  secondary_color: string
  bg_color: string
}

export type ThemeName = 'auto' | 'light' | 'dark'
