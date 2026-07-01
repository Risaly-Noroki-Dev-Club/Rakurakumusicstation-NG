// Global live stream audio element lifecycle used by App.vue.

import { nextTick, ref } from 'vue'
import { getStreamUrl } from '../api'

export function useLiveAudio() {
  // Global audio element (declared in App.vue template so browser sees it during parse)
  const audioEl = ref<HTMLAudioElement | null>(null)

  // 浏览器 autoplay 策略要求首次播放必须有用户手势触发，否则 .play() 会被静默拒绝。
  // 我们尝试自动播一次，被拒就显示一个全屏覆盖层等用户点。
  const needsTapToPlay = ref(false)

  function restartLiveStream() {
    const audio = audioEl.value
    if (!audio) return
    const wasPaused = audio.paused
    audio.pause()
    audio.removeAttribute('src')
    audio.load()
    nextTick(() => {
      const separator = getStreamUrl().includes('?') ? '&' : '?'
      audio.src = `${getStreamUrl()}${separator}t=${Date.now()}`
      audio.load()
      if (!wasPaused) audio.play().catch(() => { needsTapToPlay.value = true })
    })
  }

  function initAudio() {
    if (!audioEl.value) return
    audioEl.value.src = getStreamUrl()
    audioEl.value.load()
    requestAnimationFrame(() => {
      const p = audioEl.value?.play()
      if (p && typeof p.then === 'function') {
        p.catch(() => { needsTapToPlay.value = true })
      }
    })
  }

  function startPlaybackFromGesture() {
    if (!audioEl.value) return
    audioEl.value.play().then(() => {
      needsTapToPlay.value = false
    }).catch(() => {
      // 播放仍然失败（例如后端未运行）——关闭覆盖层，避免用户被困住
      needsTapToPlay.value = false
    })
  }

  function cleanupAudio() {
    if (!audioEl.value) return
    audioEl.value.pause()
    audioEl.value.src = ''
  }

  return {
    audioEl,
    needsTapToPlay,
    initAudio,
    restartLiveStream,
    startPlaybackFromGesture,
    cleanupAudio,
  }
}
