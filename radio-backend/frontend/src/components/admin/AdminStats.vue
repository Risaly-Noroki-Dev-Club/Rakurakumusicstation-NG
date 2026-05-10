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

loadStats()

const stats = computed(() => [
  { label: '用户', value: store.adminStats?.users || 0, icon: 'mdi-account', color: 'primary' },
  { label: '歌曲', value: store.adminStats?.songs || 0, icon: 'mdi-music-note', color: 'secondary' },
  { label: '队列', value: store.adminStats?.queue_size || 0, icon: 'mdi-playlist-music', color: 'accent' },
  { label: '歌单', value: store.adminStats?.playlists || 0, icon: 'mdi-folder-music', color: 'info' },
])

import { computed } from 'vue'
</script>

<template>
  <div>
    <div class="d-flex flex-wrap gap-4">
      <v-card
        v-for="s in stats"
        :key="s.label"
        class="flex-grow-1"
        min-width="140"
        elevation="2"
      >
        <v-card-text class="d-flex align-center py-4">
          <v-icon :color="s.color" size="32" class="mr-3">{{ s.icon }}</v-icon>
          <div>
            <div class="text-h4 font-weight-bold">{{ s.value }}</div>
            <div class="text-caption text-medium-emphasis">{{ s.label }}</div>
          </div>
        </v-card-text>
      </v-card>
    </div>
  </div>
</template>

<style scoped>
.gap-4 {
  gap: 16px;
}
</style>
