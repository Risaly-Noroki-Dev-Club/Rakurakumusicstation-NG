<script setup lang="ts">
import { computed, inject } from 'vue'
import { store, formatTime } from '../../store'
import { apiUrl, adminSkipNext, adminSkipPrev } from '../../api'
import { Music2, SkipBack, CirclePause, CirclePlay, SkipForward } from '@lucide/vue'

const audioEl = inject<import('vue').Ref<HTMLAudioElement | null>>('audioEl')

const coverSrc = computed(() => {
  if (store.playbackState.cover_url) return store.playbackState.cover_url
  if (store.playbackState.song_id > 0)
    return apiUrl('/api/songs/' + store.playbackState.song_id + '/cover')
  return ''
})

const isPlaying = computed(() => store.playbackState.status === 'playing')
const isAdmin = computed(() => store.deviceUser?.role === 'admin')

const progressPct = computed(() =>
  store.playbackState.duration_ms > 0
    ? Math.min(100, (store.displayPositionMs / store.playbackState.duration_ms) * 100)
    : 0
)

const remainingMs = computed(() =>
  Math.max(0, store.playbackState.duration_ms - store.displayPositionMs)
)

function togglePlay() {
  if (!audioEl?.value) return
  if (audioEl.value.paused) {
    audioEl.value.play().catch(() => {})
  } else {
    audioEl.value.pause()
  }
}

function onPrev() {
  if (isAdmin.value) adminSkipPrev()
}

function onNext() {
  if (isAdmin.value) adminSkipNext()
}
</script>

<template>
  <div class="lt-player-card">
    <!-- 封面 -->
    <div class="lt-cover-art">
      <img v-if="coverSrc && !store.coverLoadError" :src="coverSrc" :alt="store.playbackState.title" @error="store.coverLoadError = true" />
      <div v-else class="lt-cover-placeholder">
        <Music2 :size="56" />
      </div>
    </div>

    <!-- 曲目信息 -->
    <div class="lt-track-info">
      <div class="lt-track-title">{{ store.playbackState.title || '等待播放...' }}</div>
      <div class="lt-track-artist">{{ store.playbackState.artist || '' }}</div>
    </div>

    <!-- 进度条 -->
    <div class="lt-progress-section">
      <div class="lt-progress-track">
        <div class="lt-progress-fill" :style="{ width: progressPct + '%' }">
          <div class="lt-progress-thumb"></div>
        </div>
      </div>
      <div class="lt-progress-times">
        <span>{{ formatTime(store.displayPositionMs) }}</span>
        <span>-{{ formatTime(remainingMs) }}</span>
      </div>
    </div>

    <!-- 控件 -->
    <div class="lt-controls">
      <button class="lt-ctrl-btn" title="上一首" :disabled="!isAdmin" @click="onPrev">
        <SkipBack :size="20" />
      </button>
      <button class="lt-ctrl-btn lt-ctrl-play" :title="isPlaying ? '暂停' : '播放'" @click="togglePlay">
        <CirclePause v-if="isPlaying" :size="20" />
        <CirclePlay v-else :size="20" />
      </button>
      <button class="lt-ctrl-btn" title="下一首" :disabled="!isAdmin" @click="onNext">
        <SkipForward :size="20" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.lt-player-card {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-lg);
  padding: 24px;
  box-shadow: var(--lt-shadow-card);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.lt-cover-art {
  width: 220px;
  height: 220px;
  border-radius: var(--lt-radius-md);
  box-shadow: var(--lt-shadow-cover);
  overflow: hidden;
  flex-shrink: 0;
  background: var(--lt-btn-bg);
}

.lt-cover-art img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.lt-cover-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #2D1B69, #1A1A3E, #4A2C6E);
  color: rgba(255, 255, 255, 0.5);
}

.lt-cover-placeholder svg {
  width: 56px;
  height: 56px;
}

.lt-track-info {
  text-align: center;
  margin-top: 20px;
  width: 100%;
  max-width: 320px;
}

.lt-track-title {
  font-family: var(--lt-font-sans);
  font-size: 16px;
  font-weight: 700;
  color: var(--lt-text-primary);
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-track-artist {
  font-family: var(--lt-font-sans);
  font-size: 13px;
  font-weight: 400;
  color: var(--lt-text-secondary);
  margin-top: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-progress-section {
  width: 100%;
  max-width: 320px;
  margin-top: 18px;
}

.lt-progress-track {
  width: 100%;
  height: 4px;
  background: var(--lt-progress-track);
  border-radius: 2px;
  position: relative;
}

.lt-progress-fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: var(--lt-progress-fill);
  border-radius: 2px;
  transition: width 0.3s linear;
}

.lt-progress-thumb {
  position: absolute;
  top: 50%;
  right: -6px;
  transform: translateY(-50%);
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--lt-progress-fill);
  box-shadow: 0 0 4px rgba(92, 84, 112, 0.3);
}

.lt-progress-times {
  display: flex;
  justify-content: space-between;
  margin-top: 6px;
  font-size: 11px;
  font-weight: 400;
  color: var(--lt-text-secondary);
  font-variant-numeric: tabular-nums;
  font-family: 'SF Mono', 'Menlo', 'Consolas', monospace;
}

.lt-controls {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  margin-top: 16px;
}

.lt-ctrl-btn {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 1px solid var(--lt-divider);
  background: transparent;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: var(--lt-text-primary);
  transition: background 0.15s ease, border-color 0.15s ease, opacity 0.15s ease;
  padding: 0;
}

.lt-ctrl-btn:hover:not(:disabled) {
  background: var(--lt-btn-bg);
  border-color: var(--lt-selected-bg);
}

.lt-ctrl-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.lt-ctrl-btn svg {
  width: 20px;
  height: 20px;
}

.lt-ctrl-play {
  width: 48px;
  height: 48px;
  border: none;
  background: var(--lt-play-btn-bg);
  color: var(--lt-play-btn-icon);
}

.lt-ctrl-play:hover {
  opacity: 0.88;
}
</style>
