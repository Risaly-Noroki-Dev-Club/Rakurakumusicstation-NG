// Singleton audio element + stream lifecycle.
//
// The /stream endpoint is a live ring buffer: on admin skip/prev the server
// resyncs readers and closes the connection, so the <audio> element fires
// 'ended'/'error' — we must reconnect to the live edge (cache-busting query
// param so the browser doesn't serve a stale buffer).
//
// Autoplay policy: browsers block sound until a user gesture. The first play()
// attempt (page load) rejects with NotAllowedError → we show a resume overlay
// (store.needsPlay). Once the user clicks it, `userAuthorized` flips and every
// subsequent reconnect calls play() directly; if the browser still rejects
// (rare), we stay silent and retry on the next sync tick instead of re-showing
// the overlay — otherwise every stream hiccup would trap the user in the
// overlay loop.

import { useStore } from '@/store'

let audio: HTMLAudioElement | null = null
let reconnectNonce = 0
let errorRetries = 0
let userAuthorized = false
let connectionErrorNotified = false
let retryThrottleUntil = 0

function normalized(url: string): string {
  return new URL(url, window.location.origin).href
}

function ensureAudio(): HTMLAudioElement {
  if (audio) return audio
  const el = new Audio()
  el.preload = 'none'
  el.addEventListener('ended', () => {
    // Server closed the stream (skip/prev resync or idle timeout) → rejoin live edge.
    reconnect()
  })
  el.addEventListener('error', () => {
    errorRetries += 1
    if (errorRetries > 12) {
      if (!connectionErrorNotified) {
        connectionErrorNotified = true
        useStore.getState().addToast('直播流连接不稳定，正在自动重试…', 'warning')
      }
      retryThrottleUntil = Date.now() + 60000
      return
    }
    window.setTimeout(reconnect, 2000)
  })
  el.addEventListener('playing', () => {
    errorRetries = 0
    connectionErrorNotified = false
  })
  audio = el
  return el
}

function desiredStreamUrl(): string | null {
  const p = useStore.getState().playback
  if (!p || p.status === 'stopped') return null
  return p.streamUrl
}

export function reconnect() {
  const url = desiredStreamUrl()
  if (!url) return
  reconnectNonce += 1
  const el = ensureAudio()
  const sep = url.includes('?') ? '&' : '?'
  el.src = `${url}${sep}r=${reconnectNonce}`
  void tryPlay(el)
}

async function tryPlay(el: HTMLAudioElement) {
  const { audioPaused, needsPlay } = useStore.getState()
  if (audioPaused || needsPlay) return
  try {
    await el.play()
  } catch (e) {
    if (!userAuthorized) {
      // First contact: browser blocks sound until a user gesture.
      useStore.getState().setNeedsPlay(true)
      return
    }
    const notAllowed = e instanceof DOMException && e.name === 'NotAllowedError'
    if (notAllowed || Date.now() < retryThrottleUntil) {
      // Autoplay gate (unlikely after first unlock) or throttled retry window:
      // stay silent; the next sync tick / error event retries.
      return
    }
    // Stream-level failure on a dead element: rejoin the live edge by
    // re-pointing the src (play() alone keeps rejecting on an errored
    // element, which is what made reconnects after network drops dead).
    window.setTimeout(reconnect, 2000)
  }
}

/** Called on every store change: keeps the element pointed at the right URL. */
export function syncAudio() {
  const el = ensureAudio()
  const url = desiredStreamUrl()
  const { playback, audioPaused, needsPlay } = useStore.getState()

  if (!url) {
    el.pause()
    if (el.src) {
      el.removeAttribute('src')
      el.load()
    }
    return
  }

  const target = normalized(url)
  const current = el.src ? normalized(el.src) : ''

  if (current !== target) {
    reconnectNonce += 1
    const sep = url.includes('?') ? '&' : '?'
    el.src = `${url}${sep}r=${reconnectNonce}`
    void tryPlay(el)
    return
  }

  if (el.paused && !audioPaused && !needsPlay && playback?.status !== 'stopped') {
    void tryPlay(el)
  }
}

/** User gesture path: unblock autoplay and resume. */
export function resumeAudio() {
  userAuthorized = true
  useStore.getState().setNeedsPlay(false)
  useStore.getState().setAudioPaused(false)
  const el = ensureAudio()
  void tryPlay(el)
}

/** User explicitly pauses the stream locally. */
export function pauseAudio() {
  const el = ensureAudio()
  el.pause()
  useStore.getState().setAudioPaused(true)
}

export function isAudioPlaying(): boolean {
  return audio !== null && !audio.paused && !audio.ended
}

export function isUserAuthorized(): boolean {
  return userAuthorized
}
