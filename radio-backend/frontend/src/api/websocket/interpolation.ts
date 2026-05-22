import { store } from '../../store'

let lastAnchorPos = 0
let lastAnchorTime = 0
let rafId: number | null = null
let isInterpolating = false

export function startInterpolation(): void {
  if (isInterpolating) return
  isInterpolating = true
  tick()
}

export function stopInterpolation(): void {
  isInterpolating = false
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
}

function tick(): void {
  if (!isInterpolating) return
  const elapsed = performance.now() - lastAnchorTime
  const raw = Math.max(0, Math.floor(lastAnchorPos + elapsed))
  const duration = store.playbackState.duration_ms
  store.displayPositionMs = duration > 0 ? Math.min(raw, duration) : raw
  rafId = requestAnimationFrame(tick)
}

export function updateAnchor(positionMs: number): void {
  lastAnchorPos = Math.max(0, positionMs)
  lastAnchorTime = performance.now()
}

export function currentAnchorPosition(): number {
  return lastAnchorPos
}
