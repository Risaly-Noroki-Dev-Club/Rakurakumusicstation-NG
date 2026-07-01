<script setup lang="ts">
import { computed, nextTick, watch, ref } from 'vue'
import { store } from '../store'

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

// Reset scroll on song change
watch(() => store.playbackState.song_id, () => {
  lastScrolledIdx = -1
  if (lyricsBoxRef.value) {
    lyricsBoxRef.value.scrollTop = 0
  }
})

defineEmits<{ close: [] }>()
</script>

<template>
  <div class="am-lyrics-overlay">
    <!-- Header -->
    <div class="am-lyrics-header d-flex align-center justify-space-between px-6 py-4">
      <div>
        <div class="text-body-1 font-weight-bold am-lyrics-title">{{ store.playbackState.title }}</div>
        <div class="text-caption am-lyrics-artist">{{ store.playbackState.artist }}</div>
      </div>
      <v-btn icon variant="text" size="small" class="am-lyrics-close" @click="$emit('close')">
        <v-icon>mdi-chevron-down</v-icon>
      </v-btn>
    </div>

    <!-- Lyrics -->
    <div ref="lyricsBoxRef" class="am-lyrics-box">
      <div
        v-for="(line, idx) in store.lyricsLines"
        :key="idx"
        :class="[
          'am-lyrics-line',
          lyricActiveIdx === idx ? 'active' : 'inactive'
        ]"
        :ref="el => { if (lyricActiveIdx === idx) scrollLyric(el as HTMLElement) }"
      >
        {{ line.text }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.am-lyrics-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 200;
  display: flex;
  flex-direction: column;
  background: var(--am-surface-1);
  animation: fadeIn 0.35s var(--am-ease-emphasized);
}

.am-lyrics-header {
  flex-shrink: 0;
  position: relative;
  z-index: 1;
  border-bottom: 1px solid var(--am-divider);
  background: transparent;
}

.am-lyrics-title {
  color: var(--am-text-high);
}

.am-lyrics-artist {
  color: var(--am-text-medium);
}

.am-lyrics-close {
  color: var(--am-text-medium) !important;
}

.am-lyrics-box {
  flex: 1;
  overflow-y: auto;
  padding: 40px 32px;
  text-align: center;
  scroll-behavior: smooth;
  position: relative;
  z-index: 1;
}

.am-lyrics-line {
  padding: 10px 0;
  transition: all 0.4s var(--am-ease-smooth);
  font-family: var(--font-display);
  font-size: 1.15rem;
  line-height: 1.7;
  color: var(--am-text-disabled);
}

.am-lyrics-line.active {
  color: var(--am-text-high);
  font-weight: 700;
  font-size: 1.4rem;
}

@media (min-width: 960px) {
  .am-lyrics-line {
    font-size: 1.35rem;
  }
  .am-lyrics-line.active {
    font-size: 1.7rem;
  }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}
</style>
