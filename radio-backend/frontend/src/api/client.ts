import { store } from '../store'

export const apiBase = window.location.origin

let streamUrl = '/stream'
export let audioEngineUrl = ''

export function getStreamUrl(): string { return streamUrl }
export function getAudioEngineUrl(): string { return audioEngineUrl }
export function getBackendUrl(): string { return apiBase }

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
