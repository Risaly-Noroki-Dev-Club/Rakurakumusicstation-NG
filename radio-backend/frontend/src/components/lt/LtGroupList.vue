<script setup lang="ts">
import { computed } from 'vue'
import { store } from '../../store'

const names = computed(() => store.onlineListenerNames)
const count = computed(() => store.onlineListenerCount)
const isEmpty = computed(() => names.value.length === 0)

// 头像渐变色 — 基于姓名生成稳定色相
function avatarGradient(name: string): string {
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash)
  const h1 = Math.abs(hash) % 360
  const h2 = (h1 + 40) % 360
  return `linear-gradient(135deg, hsl(${h1}, 70%, 60%), hsl(${h2}, 70%, 55%))`
}

function initial(name: string): string {
  return name.charAt(0).toUpperCase() || '?'
}
</script>

<template>
  <div class="lt-group-list-card">
    <div class="lt-group-header">
      <svg class="lt-back-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6"></polyline>
      </svg>
      <span class="lt-group-title">Group List</span>
      <span class="lt-group-count">{{ count }}</span>
    </div>

    <div v-if="isEmpty" class="lt-empty">暂无其他听众</div>

    <div
      v-for="(name, idx) in names"
      :key="idx"
      class="lt-listener-row"
    >
      <div class="lt-listener-avatar" :style="{ background: avatarGradient(name) }">
        {{ initial(name) }}
      </div>
      <span class="lt-listener-name">{{ name }}</span>
      <span v-if="idx === 0" class="lt-next-badge">Next</span>
    </div>
  </div>
</template>

<style scoped>
.lt-group-list-card {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-md);
  padding: 16px;
  box-shadow: var(--lt-shadow-subtle);
  flex-shrink: 0;
  max-height: 250px;
  overflow-y: auto;
}

.lt-group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
}

.lt-back-icon {
  width: 18px;
  height: 18px;
  color: var(--lt-text-primary);
  flex-shrink: 0;
}

.lt-group-title {
  font-family: var(--lt-font-serif);
  font-size: 18px;
  font-weight: 700;
  color: var(--lt-text-primary);
}

.lt-group-count {
  margin-left: auto;
  font-size: 12px;
  font-weight: 600;
  color: var(--lt-text-secondary);
  background: var(--lt-btn-bg);
  padding: 2px 8px;
  border-radius: 10px;
}

.lt-empty {
  font-size: 13px;
  color: var(--lt-text-muted);
  padding: 8px 4px;
  text-align: center;
}

.lt-listener-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 4px;
  border-bottom: 1px solid var(--lt-divider);
}

.lt-listener-row:last-child {
  border-bottom: none;
}

.lt-listener-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  color: #fff;
  flex-shrink: 0;
}

.lt-listener-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--lt-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-next-badge {
  margin-left: auto;
  background: var(--lt-accent);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 10px;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  flex-shrink: 0;
}
</style>
