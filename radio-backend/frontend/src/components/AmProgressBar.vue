<script setup lang="ts">
import { computed } from 'vue'
import { store, formatTime } from '../store'

const progressPct = computed(() =>
  store.playbackState.duration_ms > 0
    ? Math.min(100, (store.displayPositionMs / store.playbackState.duration_ms) * 100)
    : 0
)
</script>

<template>
  <div class="am-progress-bar">
    <div class="d-flex justify-space-between mb-1">
      <span class="am-time">{{ formatTime(store.displayPositionMs) }}</span>
      <span class="am-time">{{ formatTime(store.playbackState.duration_ms) }}</span>
    </div>
    <div class="am-progress-track">
      <div
        class="am-progress-fill"
        :style="{ width: progressPct + '%' }"
      />
    </div>
  </div>
</template>

<style scoped>
.am-progress-bar {
  width: 100%;
}

.am-time {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--am-text-medium);
  letter-spacing: 0.05em;
}

.am-progress-track {
  width: 100%;
  height: 4px;
  background: var(--am-divider);
  border-radius: 2px;
  overflow: hidden;
  cursor: pointer;
}

.am-progress-fill {
  height: 100%;
  background: var(--am-primary);
  border-radius: 2px;
  transition: width 0.3s linear;
}
</style>
