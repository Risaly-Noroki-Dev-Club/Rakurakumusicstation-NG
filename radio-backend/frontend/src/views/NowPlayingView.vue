<script setup lang="ts">
import { computed, inject } from 'vue'
import { store } from '../store'
import { apiUrl, adminSkipNext, adminSkipPrev } from '../api'
import AmProgressBar from '../components/AmProgressBar.vue'
import LyricsView from '../components/LyricsView.vue'

const audioEl = inject<import('vue').Ref<HTMLAudioElement | null>>('audioEl')

const coverSrc = computed(() => {
  if (store.playbackState.cover_url) return store.playbackState.cover_url
  if (store.playbackState.song_id > 0)
    return apiUrl('/api/songs/' + store.playbackState.song_id + '/cover')
  return ''
})

const isPlaying = computed(() => store.playbackState.status === 'playing')
const hasLyrics = computed(() => store.lyricsLines.length > 0)
const isAdmin = computed(() => store.deviceUser?.role === 'admin')

function toggleLyrics() {
  if (!hasLyrics.value) return
  store.showLyrics = !store.showLyrics
}

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
  <div class="am-now-playing">
    <!-- Desktop: side-by-side / Mobile: stacked -->
    <div class="am-player-layout" :class="{ desktop: store.isDesktop }">
      <!-- Cover -->
      <div class="am-cover-section">
        <div class="am-cover-wrapper" :class="{ 'is-playing': isPlaying }">
          <v-img
            :src="coverSrc"
            :alt="store.playbackState.title"
            cover
            class="am-cover-img"
            @error="store.coverLoadError = true"
            @load="store.coverLoadError = false"
          >
            <template #placeholder>
              <div class="d-flex align-center justify-center fill-height" style="background: var(--am-surface-2);">
                <v-icon size="64" color="primary" style="opacity: 0.5">mdi-music-note</v-icon>
              </div>
            </template>
          </v-img>
        </div>
      </div>

      <!-- Info & Controls -->
      <div class="am-info-section">
        <div class="am-track-info text-center" :class="{ 'text-left': store.isDesktop }">
          <h1 class="am-track-title text-h4 font-weight-extrabold text-truncate">
            {{ store.playbackState.title || '等待播放...' }}
          </h1>
          <p class="am-track-artist text-subtitle-1 text-medium-emphasis mt-2 text-truncate font-weight-medium">
            {{ store.playbackState.artist || '' }}
          </p>
        </div>

        <!-- Progress Bar -->
        <div class="am-progress-wrapper mt-8">
          <AmProgressBar />
        </div>

        <!-- Controls -->
        <div class="am-controls d-flex align-center justify-center mt-8" :class="{ 'justify-start': store.isDesktop }">
          <v-btn
            icon
            variant="text"
            color="medium-emphasis"
            class="mx-3 am-ctrl-btn"
            :disabled="!isAdmin"
            @click="onPrev"
          >
            <v-icon size="28">mdi-skip-previous</v-icon>
          </v-btn>

          <v-btn
            icon
            size="x-large"
            color="primary"
            elevation="0"
            class="mx-4 am-play-btn"
            @click="togglePlay"
          >
            <v-icon size="36">{{ isPlaying ? 'mdi-pause' : 'mdi-play' }}</v-icon>
          </v-btn>

          <v-btn
            icon
            variant="text"
            color="medium-emphasis"
            class="mx-3 am-ctrl-btn"
            :disabled="!isAdmin"
            @click="onNext"
          >
            <v-icon size="28">mdi-skip-next</v-icon>
          </v-btn>
        </div>

        <!-- Action row -->
        <div class="d-flex align-center justify-center mt-6 gap-3" :class="{ 'justify-start': store.isDesktop }">
          <v-btn
            variant="tonal"
            density="comfortable"
            :color="hasLyrics ? 'primary' : undefined"
            :disabled="!hasLyrics"
            @click="toggleLyrics"
            rounded="xl"
            size="small"
          >
            <v-icon left size="18" class="mr-1">mdi-text-box-outline</v-icon>
            {{ hasLyrics ? '歌词' : '无歌词' }}
          </v-btn>
        </div>
      </div>
    </div>

    <!-- Lyrics Overlay -->
    <LyricsView v-if="store.showLyrics" @close="store.showLyrics = false" />
  </div>
</template>

<style scoped>
.am-now-playing {
  position: relative;
  min-height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 16px 0;
}

.am-player-layout {
  width: 100%;
  max-width: 600px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 32px;
  z-index: 1;
  animation: slideUp 0.6s var(--am-ease-emphasized);
}

.am-player-layout.desktop {
  max-width: 1000px;
  flex-direction: row;
  align-items: center;
  gap: 64px;
}

.am-cover-section {
  flex-shrink: 0;
}

.am-cover-wrapper {
  width: 280px;
  height: 280px;
  border-radius: var(--am-radius-xl);
  overflow: hidden;
  position: relative;
  box-shadow: var(--am-shadow-16);
  transition: transform 0.6s var(--am-ease-emphasized), box-shadow 0.6s var(--am-ease-emphasized);
}

.am-cover-wrapper.is-playing {
  transform: scale(1.02);
}

.am-cover-img {
  width: 100%;
  height: 100%;
  border-radius: var(--am-radius-xl);
}

.am-info-section {
  flex: 1;
  width: 100%;
  max-width: 480px;
}

.am-track-title {
  font-family: var(--font-display);
  line-height: 1.15;
  letter-spacing: -0.02em;
}

.am-track-artist {
  font-family: var(--font-display);
  letter-spacing: 0.01em;
}

.am-progress-wrapper {
  width: 100%;
}

.am-play-btn {
  border-radius: var(--am-radius-full) !important;
  min-width: 64px !important;
  min-height: 64px !important;
  width: 64px !important;
  height: 64px !important;
}

.am-ctrl-btn {
  opacity: 0.7;
  transition: opacity 0.2s ease, transform 0.2s var(--am-ease-spring);
}

.am-ctrl-btn:hover:not(:disabled) {
  opacity: 1;
  transform: scale(1.08);
}

.gap-3 {
  gap: 12px;
}

/* Desktop overrides */
@media (min-width: 960px) {
  .am-cover-wrapper {
    width: 400px;
    height: 400px;
  }

  .am-track-title {
    font-size: 2.5rem !important;
  }

  .am-track-artist {
    font-size: 1.25rem !important;
  }

  .am-play-btn {
    min-width: 72px !important;
    min-height: 72px !important;
    width: 72px !important;
    height: 72px !important;
  }

  .am-play-btn .v-icon {
    size: 40px;
  }
}

@keyframes slideUp {
  from { transform: translateY(24px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
