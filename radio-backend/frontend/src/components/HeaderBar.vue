<script setup lang="ts">
import { computed } from 'vue'
import { store, cycleTheme, THEMES } from '../store'
import { openAuth, logout } from '../api'

const themeIcon = computed(() => {
  const icons: Record<string, string> = { auto: '🌓', light: '☀️', dark: '🌙' }
  return icons[THEMES[store.themeIdx]] || '🌓'
})
</script>

<template>
  <header>
    <h1><span>🎵</span><span>{{ store.stationName }}</span></h1>
    <div>
      <button class="theme-toggle" title="切换主题" @click="cycleTheme">{{ themeIcon }}</button>
      <span v-if="store.useFileMode" class="mode-badge mode-file">推文件模式</span>
      <span v-if="store.currentUser" class="user-display">{{ store.currentUser.username }}</span>
      <button v-if="!store.currentUser" class="btn btn-secondary btn-small" @click="openAuth">登录</button>
      <button v-if="store.currentUser" class="btn btn-secondary btn-small" @click="logout">退出</button>
    </div>
  </header>
</template>
