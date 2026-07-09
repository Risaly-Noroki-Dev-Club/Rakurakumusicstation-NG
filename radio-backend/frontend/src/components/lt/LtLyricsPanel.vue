<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { store } from '../../store'

const lyricActiveIdx = computed(() => {
  if (store.lyricsLines.length === 0) return -1
  const pos = store.displayPositionMs
  let idx = -1
  for (let i = store.lyricsLines.length - 1; i >= 0; i--) {
    if (store.lyricsLines[i].timeMs <= pos) {
      idx = i
      break
    }
  }
  return idx
})

const lyricsBoxRef = ref<HTMLElement | null>(null)
let lastScrolledIdx = -1

function scrollLyric(el: HTMLElement | null) {
  if (lyricActiveIdx.value >= 0 && lyricActiveIdx.value !== lastScrolledIdx) {
    lastScrolledIdx = lyricActiveIdx.value
    nextTick(() => {
      if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' })
    })
  }
}

// 歌曲切换时重置滚动位置
watch(() => store.playbackState.song_id, () => {
  lastScrolledIdx = -1
  if (lyricsBoxRef.value) {
    lyricsBoxRef.value.scrollTop = 0
  }
})

const hasLyrics = computed(() => store.lyricsLines.length > 0)

function distanceClass(idx: number): string {
  if (lyricActiveIdx.value === idx) return 'active'
  const d = Math.abs(lyricActiveIdx.value - idx)
  if (d <= 2) return 'near'
  return 'far'
}
</script>

<template>
  <aside class="lt-lyrics-panel">
    <div class="lt-lyrics-fade-top"></div>
    <div class="lt-lyrics-fade-bottom"></div>

    <div v-if="!hasLyrics" class="lt-lyrics-empty">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M9 18V5l12-2v13" />
        <circle cx="6" cy="18" r="3" />
        <circle cx="18" cy="16" r="3" />
      </svg>
      <span>暂无歌词</span>
    </div>

    <div v-else ref="lyricsBoxRef" class="lt-lyrics-content">
      <div class="lt-lyrics-note">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 18V5l12-2v13" />
          <circle cx="6" cy="18" r="3" />
          <circle cx="18" cy="16" r="3" />
        </svg>
      </div>
      <div
        v-for="(line, idx) in store.lyricsLines"
        :key="idx"
        :class="['lt-lyric-line', distanceClass(idx)]"
        :ref="el => { if (lyricActiveIdx === idx) scrollLyric(el as HTMLElement) }"
      >
        {{ line.text }}
      </div>
      <div class="lt-lyrics-spacer"></div>
    </div>
  </aside>
</template>

<style scoped>
.lt-lyrics-panel {
  width: 300px;
  min-width: 260px;
  max-width: 380px;
  background: var(--lt-sidebar-bg);
  display: flex;
  flex-direction: column;
  align-items: center;
  overflow: hidden;
  position: relative;
  /* 规则3: 响应式内边距 */
  padding: clamp(16px, 4vw, 32px) clamp(8px, 3vw, 20px);
  flex-shrink: 0;
}

.lt-lyrics-fade-top,
.lt-lyrics-fade-bottom {
  position: absolute;
  left: 0;
  right: 0;
  height: 60px;
  pointer-events: none;
  z-index: 2;
}

.lt-lyrics-fade-top {
  top: 0;
  background: linear-gradient(to bottom, var(--lt-sidebar-bg) 0%, transparent 100%);
}

.lt-lyrics-fade-bottom {
  bottom: 0;
  background: linear-gradient(to top, var(--lt-sidebar-bg) 0%, transparent 100%);
}

.lt-lyrics-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--lt-text-muted);
  font-size: 13px;
}

.lt-lyrics-empty svg {
  width: 32px;
  height: 32px;
  opacity: 0.4;
}

.lt-lyrics-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  width: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  /* 规则3: 调整内边距，窄屏时减少 padding 释放空间 */
  padding: clamp(16px, 5vw, 40px) clamp(4px, 2vw, 8px);
  scroll-behavior: smooth;
  mask-image: linear-gradient(transparent 0%, black 8%, black 92%, transparent 100%);
  -webkit-mask-image: linear-gradient(transparent 0%, black 8%, black 92%, transparent 100%);
}

.lt-lyrics-note {
  margin-bottom: 12px;
  opacity: 0.25;
}

.lt-lyrics-note svg {
  width: 20px;
  height: 20px;
  color: var(--lt-text-secondary);
}

.lt-lyric-line {
  font-family: var(--lt-font-lyric);
  text-align: center;
  padding: 8px 0;
  line-height: 1.6;
  transition: font-size 0.3s ease, color 0.3s ease, opacity 0.3s ease;
  width: 100%;
  /* 规则1: 响应式缩小字号 */
  font-size: clamp(12px, 2.2vw, 15px);
  /* 规则2: 允许换行 */
  overflow-wrap: break-word;
  word-break: break-word;
  overflow: hidden;
  hyphens: auto;
}

.lt-lyric-line.active {
  font-size: clamp(14px, 2.5vw, 18px);
  font-weight: 700;
  color: var(--lt-text-primary);
  opacity: 1;
}

.lt-lyric-line.near {
  font-size: 15px;
  font-weight: 400;
  color: var(--lt-text-secondary);
  opacity: 0.55;
}

.lt-lyric-line.far {
  font-size: 15px;
  font-weight: 400;
  color: var(--lt-text-secondary);
  opacity: 0.3;
}

.lt-lyrics-spacer {
  height: 40px;
  flex-shrink: 0;
}
</style>
