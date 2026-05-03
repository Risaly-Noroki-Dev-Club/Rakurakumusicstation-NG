<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { store, toast } from './store'
import {
  loadStationInfo, doSetup, loadCurrentUser, loadMyPlaylists,
  connectWebSocket, startPollers, stopPollers, getWs
} from './api'
import HeaderBar from './components/HeaderBar.vue'
import NavTabs from './components/NavTabs.vue'
import AuthModal from './components/AuthModal.vue'
import ToastContainer from './components/ToastContainer.vue'

let queuePoller: ReturnType<typeof setInterval> | null = null

function handleSetup() {
  doSetup(store.setupUsername.trim(), store.setupPassword)
}

async function init() {
  await loadStationInfo()
  connectWebSocket()
  if (store.token) {
    await loadCurrentUser()
    if (store.currentUser) loadMyPlaylists()
  }
  const pollers = startPollers()
  queuePoller = pollers.queuePoller
}

onMounted(init)

onUnmounted(() => {
  if (queuePoller) stopPollers(queuePoller)
  if (getWs()) getWs()!.close()
})

// Service Worker
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
  <div v-if="store.needsSetup" class="setup-overlay">
    <div class="card" style="width:400px;max-width:90vw">
      <h2>🎵 初次设置 — 创建管理员账户</h2>
      <p style="color:var(--text-muted);margin-bottom:16px">这是电台的首次运行。请创建管理员账户来管理电台。</p>
      <div class="form-group">
        <label>管理员用户名</label>
        <input type="text" v-model="store.setupUsername" autocomplete="off" placeholder="3-32个字符" @keyup.enter="handleSetup">
      </div>
      <div class="form-group">
        <label>密码</label>
        <input type="password" v-model="store.setupPassword" autocomplete="new-password" placeholder="至少6个字符" @keyup.enter="handleSetup">
      </div>
      <button class="btn btn-primary" @click="handleSetup" style="width:100%;margin-top:8px">创建管理员账户</button>
      <div v-if="store.setupError" style="color:var(--danger);margin-top:8px;font-size:0.85em">{{ store.setupError }}</div>
    </div>
  </div>

  <div v-else class="app">
    <HeaderBar />
    <NavTabs />
    <router-view />
    <AuthModal />
    <ToastContainer />
  </div>
</template>
