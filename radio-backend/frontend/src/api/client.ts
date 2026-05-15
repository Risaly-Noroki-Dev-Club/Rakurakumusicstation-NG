import { store } from '../store'

export const apiBase = window.location.origin
export const appBase = import.meta.env.BASE_URL

let streamUrl = appPath('/stream')
export let audioEngineUrl = ''

export function getStreamUrl(): string { return streamUrl }
export function getAudioEngineUrl(): string { return audioEngineUrl }
export function getBackendUrl(): string { return apiBase }

export function appPath(path: string): string {
  const base = appBase.endsWith('/') ? appBase : appBase + '/'
  const cleanPath = path.startsWith('/') ? path.slice(1) : path
  return base + cleanPath
}

export function apiUrl(path: string): string {
  return apiBase + appPath(path)
}

export function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  return fetch(apiUrl(path), {
    credentials: 'same-origin',
    ...init,
    headers: init.headers,
  })
}

export function setStreamUrl(url: string): void {
  streamUrl = url
}

export function setAudioEngineUrl(url: string): void {
  audioEngineUrl = url
}

export function resolveStreamUrlFromStation(info: { stream_url?: string }): void {
  if (info.stream_url) {
    streamUrl = info.stream_url
    try {
      const u = new URL(info.stream_url, window.location.origin)
      audioEngineUrl = u.origin
    } catch { /* ignore */ }
  }
}
