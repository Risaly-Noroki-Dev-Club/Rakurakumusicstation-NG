import { useEffect, useState } from 'react'
import type { Playback } from '@/store'

/**
 * Smooth client-side playback position.
 *
 * The server sends position_ms + timestamp_ms every 500 ms; between frames we
 * extrapolate with Date.now() (same epoch as the server timestamp). While the
 * status is not "playing" the raw position is returned.
 */
export function usePlaybackClock(playback: Playback | null): number {
  const [now, setNow] = useState(() => Date.now())

  useEffect(() => {
    if (!playback || playback.status !== 'playing') return
    setNow(Date.now())
    const id = window.setInterval(() => setNow(Date.now()), 250)
    return () => window.clearInterval(id)
  }, [playback?.status, playback?.timestampMs, playback?.positionMs, playback?.durationMs])

  if (!playback) return 0
  if (playback.status !== 'playing') {
    return Math.max(0, Math.min(playback.positionMs, playback.durationMs || playback.positionMs))
  }
  const elapsed = Math.max(0, now - playback.timestampMs)
  const estimate = playback.positionMs + elapsed
  return playback.durationMs > 0 ? Math.min(estimate, playback.durationMs) : estimate
}

/** Current lyrics line index for a given position (last line whose time_ms <= pos). */
export function activeLyricsIndex(lines: { time_ms: number }[] | null | undefined, positionMs: number): number {
  if (!lines || lines.length === 0) return -1
  let idx = -1
  for (let i = 0; i < lines.length; i += 1) {
    if (lines[i].time_ms <= positionMs) idx = i
    else break
  }
  return idx
}
