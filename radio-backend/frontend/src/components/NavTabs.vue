<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { store } from '../store'

const route = useRoute()

const tabs = [
  { name: 'player', label: '▶️ 播放', show: true },
  { name: 'queue', label: '📋 队列', show: true },
  { name: 'library', label: '🔍 曲库', show: true },
  { name: 'admin', label: '⚙️ 管理', show: false },
]

const currentTab = computed(() => {
  const p = route.path
  if (p === '/') return 'player'
  if (p.startsWith('/player')) return 'player'
  if (p.startsWith('/queue')) return 'queue'
  if (p.startsWith('/library')) return 'library'
  if (p.startsWith('/admin')) return 'admin'
  return 'player'
})

const isAdmin = computed(() => store.currentUser?.role === 'admin')
</script>

<template>
  <div class="tab-bar">
    <router-link
      v-for="t in tabs"
      :key="t.name"
      v-show="t.show || (t.name === 'admin' && isAdmin)"
      :to="'/' + t.name"
      :class="{ tab: true, active: currentTab === t.name }"
    >{{ t.label }}</router-link>
  </div>
</template>
