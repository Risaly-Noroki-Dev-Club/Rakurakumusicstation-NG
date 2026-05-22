// Compatibility aggregation for WebSocket-related frontend helpers.
// New code should prefer the focused modules under `api/websocket/`.

import { store } from '../store'
import { refreshQueue } from './queue'
import {
  closeWebSocket,
  connectWebSocket,
  getWs
} from './websocket/connection'
import { refreshPlaybackPoll } from './websocket/fallbackPolling'
import { stopInterpolation } from './websocket/interpolation'

let playbackPoller: ReturnType<typeof setInterval> | null = null

export function startPollers(): { queuePoller: ReturnType<typeof setInterval>, playbackPoller: ReturnType<typeof setInterval> } {
  playbackPoller = setInterval(refreshPlaybackPoll, 2000)
  const queuePoller = setInterval(refreshQueue, 5000)
  return { queuePoller, playbackPoller }
}

export function stopPollers(qp: ReturnType<typeof setInterval>): void {
  stopInterpolation()
  if (playbackPoller) clearInterval(playbackPoller)
  if (qp) clearInterval(qp)
  closeWebSocket()
}

export { connectWebSocket, getWs }

export { refreshPlaybackPoll }
export { volumeDown, volumeUp } from '../app/audioControls'
export { cleanupFilePlayback, switchPlaybackMode } from '../app/filePlayback'
