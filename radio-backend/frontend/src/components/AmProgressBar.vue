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
    <div class="d-flex justify-space-between mb-2">
      <span class="am-time">{{ formatTime(store.displayPositionMs) }}</span>
      <span class="am-time">{{ formatTime(store.playbackState.duration_ms) }}</span>
    </div>
    <div class="am-progress-track">
      <div
        class="am-progress-fill"
        :style="{ width: progressPct + '%' }"
      />
      <div
        class="am-progress-thumb"
        :style="{ left: progressPct + '%' }"
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
  font-size: 0.72rem;
  font-weight: 500;
  color: var(--am-text-medium);
  letter-spacing: 0.04em;
}

.am-progress-track {
  width: 100%;
  height: 6px;
  background: var(--am-surface-3);
  border-radius: var(--am-radius-full);
  overflow: visible;
  cursor: pointer;
  position: relative;
}

.am-progress-fill {
  height: 100%;
  background: var(--am-primary);
  border-radius: var(--am-radius-full);
  transition: width 0.3s var(--am-ease-smooth);
  position: relative;
}

.am-progress-thumb {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--am-primary);
  transition: left 0.3s var(--am-ease-smooth), transform 0.15s var(--am-ease-spring);
  opacity: 0;
}

.am-progress-track:hover .am-progress-thumb {
  opacity: 1;
}

.am-progress-track:hover .am-progress-thumb {
  transform: translate(-50%, -50%) scale(1.2);
}
</style>
