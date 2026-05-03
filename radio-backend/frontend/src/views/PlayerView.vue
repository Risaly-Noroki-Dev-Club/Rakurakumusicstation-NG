<script setup lang="ts">
import { ref, computed, nextTick, onMounted } from 'vue'
import { store, formatTime } from '../store'
import { getBackendUrl, getStreamUrl, switchPlaybackMode, volumeDown, volumeUp } from '../api'

const audioEl = ref<HTMLAudioElement | null>(null)
const lyricsBoxRef = ref<HTMLElement | null>(null)

const coverSrc = computed(() => {
  if (store.playbackState.cover_url) return store.playbackState.cover_url
  if (store.playbackState.song_id > 0)
    return getBackendUrl() + '/api/songs/' + store.playbackState.song_id + '/cover'
  return ''
})

const showCover = computed(() => {
  return !!(store.playbackState.cover_url || store.playbackState.song_id > 0)
})

const titleDisplay = computed(() => store.playbackState.title || '等待播放...')
const artistDisplay = computed(() => store.playbackState.artist || '')

const progressPct = computed(() =>
  store.playbackState.duration_ms > 0
    ? Math.min(100, (store.playbackState.position_ms / store.playbackState.duration_ms) * 100)
    : 0
)

const currentTimeFormatted = computed(() => formatTime(store.playbackState.position_ms))
const totalTimeFormatted = computed(() => formatTime(store.playbackState.duration_ms))

const lyricActiveIdx = computed(() => {
  if (store.lyricsLines.length === 0) return -1
  const pos = store.playbackState.position_ms
  let idx = -1
  for (let i = store.lyricsLines.length - 1; i >= 0; i--) {
    if (store.lyricsLines[i].timeMs <= pos) {
      idx = i
      break
    }
  }
  return idx
})

let lastScrolledIdx = -1
function scrollLyric(el: HTMLElement | null) {
  if (lyricActiveIdx.value >= 0 && lyricActiveIdx.value !== lastScrolledIdx) {
    lastScrolledIdx = lyricActiveIdx.value
    nextTick(() => {
      if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    })
  }
}

function onSwitchPlaybackMode() {
  if (audioEl.value) switchPlaybackMode(audioEl.value)
}

function onVolDown() {
  if (audioEl.value) volumeDown(audioEl.value)
}

function onVolUp() {
  if (audioEl.value) volumeUp(audioEl.value)
}

onMounted(() => {
  if (audioEl.value) {
    audioEl.value.src = getStreamUrl()
    audioEl.value.load()
    audioEl.value.play().catch(() => {})
  }
})
</script>

<template>
  <div>
    <div class="card now-playing">
      <div class="player-layout">
        <div :class="['cover-container', { show: showCover && !store.coverLoadError }]">
          <img :src="coverSrc" alt="Album art"
               @error="store.coverLoadError = true"
               @load="store.coverLoadError = false">
        </div>
        <div class="player-info">
          <div class="title">{{ titleDisplay }}</div>
          <div class="artist">{{ artistDisplay }}</div>
        </div>
      </div>
      <div class="progress-bar">
        <div class="fill" :style="{ width: progressPct + '%' }"></div>
      </div>
      <div class="time-row">
        <span>{{ currentTimeFormatted }}</span>
        <span>{{ totalTimeFormatted }}</span>
      </div>
      <audio ref="audioEl" controls autoplay style="width:100%; margin:8px 0;"></audio>
      <div class="controls">
        <button class="btn btn-primary btn-small" @click="onSwitchPlaybackMode" title="切换推流/推文件模式">🔄 切换模式</button>
        <button class="btn btn-secondary btn-small" @click="onVolDown">🔉</button>
        <button class="btn btn-secondary btn-small" @click="onVolUp">🔊</button>
      </div>
    </div>

    <div :class="['card', { hidden: store.lyricsLines.length === 0 }]">
      <h2>📝 歌词</h2>
      <div class="lyrics-box" ref="lyricsBoxRef">
        <div v-if="store.lyricsLines.length === 0" class="line inactive">暂无歌词</div>
        <div v-for="(line, idx) in store.lyricsLines" :key="idx"
             :class="['line', lyricActiveIdx === idx ? 'active' : 'inactive']"
             :ref="el => { if (lyricActiveIdx === idx) scrollLyric(el as HTMLElement) }">
          {{ line.text }}
        </div>
      </div>
    </div>
  </div>
</template>
