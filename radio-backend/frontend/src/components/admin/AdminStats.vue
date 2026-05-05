<script setup lang="ts">
import { store } from '../../store'
import { getBackendUrl } from '../../api'

async function loadStats() {
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/stats')
    const d = await r.json()
    if (d.success) store.adminStats = d.data
  } catch { /* ignore */ }
}

const emit = defineEmits<{ load: [] }>()
loadStats()
</script>

<template>
  <div class="card admin-panel">
    <h2>📊 系统统计</h2>
    <div v-if="store.adminStats">
      👥 用户: {{ store.adminStats.users || 0 }} &nbsp;|&nbsp;
      🎵 歌曲: {{ store.adminStats.songs || 0 }} &nbsp;|&nbsp;
      📋 队列: {{ store.adminStats.queue_size || 0 }} &nbsp;|&nbsp;
      📁 歌单: {{ store.adminStats.playlists || 0 }}
    </div>
  </div>
</template>
