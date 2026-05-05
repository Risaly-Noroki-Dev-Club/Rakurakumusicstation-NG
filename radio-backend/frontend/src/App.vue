<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { store } from './store'
import {
  loadStationInfo, loadDeviceUser,
  connectWebSocket, startPollers, stopPollers, getWs
} from './api'
import HeaderBar from './components/HeaderBar.vue'
import NavTabs from './components/NavTabs.vue'
import ToastContainer from './components/ToastContainer.vue'

let queuePoller: ReturnType<typeof setInterval> | null = null

async function init() {
  await loadStationInfo()
  connectWebSocket()
  await loadDeviceUser()
  const pollers = startPollers()
  queuePoller = pollers.queuePoller
}

onMounted(init)

onUnmounted(() => {
  if (queuePoller) stopPollers(queuePoller)
  if (getWs()) getWs()!.close()
})

if ('serviceWorker' in navigator) {
  window.addEventListener('load', async () => {
    try {
      await navigator.serviceWorker.register('/sw.js')
      console.log('Service Worker 注册成功')
    } catch (error) {
      console.error('Service Worker 注册失败:', error)
    }
  })
}
</script>

<template>
  <div class="app">
    <HeaderBar />
    <NavTabs />
    <router-view />
    <ToastContainer />
  </div>
</template>
