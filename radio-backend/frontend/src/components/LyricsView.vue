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
  <div
    class="am-lyrics-overlay"
    :style="{ backgroundColor: store.extractedColor }"
  >
    <div class="am-lyrics-header d-flex align-center justify-space-between px-4 py-3">
      <div>
        <div class="text-body-1 font-weight-bold text-inverse">{{ store.playbackState.title }}</div>
        <div class="text-caption text-inverse opacity-70">{{ store.playbackState.artist }}</div>
      </div>
      <v-btn icon variant="text" color="white" @click="$emit('close')">
        <v-icon>mdi-chevron-down</v-icon>
      </v-btn>
    </div>

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
  animation: fadeIn 0.3s ease;
}

.am-lyrics-header {
  flex-shrink: 0;
  border-bottom: 1px solid rgba(255,255,255,0.1);
}

.text-inverse {
  color: #ffffff;
}

.opacity-70 {
  opacity: 0.7;
}

.am-lyrics-box {
  flex: 1;
  overflow-y: auto;
  padding: 32px 24px;
  text-align: center;
  scroll-behavior: smooth;
}

.am-lyrics-line {
  padding: 10px 0;
  transition: all 0.3s ease;
  font-family: var(--font-display);
  font-size: 1.1rem;
  line-height: 1.6;
}

.am-lyrics-line.inactive {
  color: rgba(255,255,255,0.5);
}

.am-lyrics-line.active {
  color: #ffffff;
  font-weight: 700;
  font-size: 1.35rem;
  text-shadow: 0 2px 8px rgba(0,0,0,0.3);
}

@media (min-width: 960px) {
  .am-lyrics-line {
    font-size: 1.3rem;
  }
  .am-lyrics-line.active {
    font-size: 1.6rem;
  }
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}
</style>
