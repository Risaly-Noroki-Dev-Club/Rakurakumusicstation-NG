<script setup lang="ts">
import { computed, inject } from 'vue'
import { useRouter } from 'vue-router'
import { store } from '../store'
import { apiUrl } from '../api'

const router = useRouter()
const audioEl = inject<import('vue').Ref<HTMLAudioElement | null>>('audioEl')

const coverSrc = computed(() => {
  if (store.playbackState.cover_url) return store.playbackState.cover_url
  if (store.playbackState.song_id > 0)
    return apiUrl('/api/songs/' + store.playbackState.song_id + '/cover')
  return ''
})

const isPlaying = computed(() => store.playbackState.status === 'playing')

function goToPlayer() {
  router.push('/')
}

function togglePlay(e: Event) {
  e.stopPropagation()
  if (!audioEl?.value) return
  if (audioEl.value.paused) {
    audioEl.value.play().catch(() => {})
  } else {
    audioEl.value.pause()
  }
}
</script>

<template>
  <div
    class="am-mini-player d-flex align-center px-4 py-2"
    @click="goToPlayer"
  >
    <v-avatar size="48" rounded="lg" class="flex-shrink-0 mr-3">
      <v-img
        :src="coverSrc"
        alt="cover"
        cover
        @error="store.coverLoadError = true"
        @load="store.coverLoadError = false"
      >
        <template #placeholder>
          <div class="d-flex align-center justify-center fill-height bg-grey-lighten-3">
            <v-icon color="grey">mdi-music-note</v-icon>
          </div>
        </template>
      </v-img>
    </v-avatar>

    <div class="flex-grow-1 min-width-0 mr-3">
      <div class="text-body-2 font-weight-semibold text-truncate">
        {{ store.playbackState.title || '等待播放...' }}
      </div>
      <div class="text-caption text-medium-emphasis text-truncate">
        {{ store.playbackState.artist || '' }}
      </div>
    </div>

    <div class="d-flex align-center gap-2 flex-shrink-0">
      <v-btn
        icon
        variant="text"
        density="comfortable"
        color="primary"
        @click="togglePlay"
      >
        <v-icon size="28">{{ isPlaying ? 'mdi-pause' : 'mdi-play' }}</v-icon>
      </v-btn>
    </div>
  </div>
</template>

<style scoped>
.am-mini-player {
  position: fixed;
  bottom: 64px;
  left: 0;
  right: 0;
  z-index: 99;
  background: var(--am-surface);
  border-top: 1px solid var(--am-divider);
  box-shadow: var(--am-shadow-4);
  cursor: pointer;
  animation: am-mini-enter 0.52s var(--am-ease-emphasized) both;
  transition:
    background-color 0.28s var(--am-ease-emphasized),
    box-shadow 0.28s var(--am-ease-emphasized),
    transform 0.28s var(--am-ease-spring);
}

.am-mini-player:hover {
  background: var(--am-surface-2);
  box-shadow: var(--am-shadow-8);
  transform: translateY(-2px) scale(1.002);
}

@media (min-width: 960px) {
  .am-mini-player {
    bottom: 0;
    left: 80px;
    right: 0;
  }
}

.gap-2 {
  gap: 8px;
}

.min-width-0 {
  min-width: 0;
}

.font-weight-semibold {
  font-weight: 600;
}

@keyframes am-mini-enter {
  from {
    opacity: 0;
    transform: translateY(14px) scale(0.99);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

@media (prefers-reduced-motion: reduce) {
  .am-mini-player {
    animation: none;
  }

  .am-mini-player:hover {
    transform: none;
  }
}
</style>
