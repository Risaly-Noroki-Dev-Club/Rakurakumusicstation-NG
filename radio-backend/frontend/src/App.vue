<script setup lang="ts">
import { onMounted, onUnmounted, computed, watch, ref, provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { store, THEMES } from './store'
import {
  loadStationInfo, loadDeviceUser,
  connectWebSocket, startPollers, stopPollers, getWs,
  getStreamUrl
} from './api'
import MiniPlayer from './components/MiniPlayer.vue'

const route = useRoute()
const router = useRouter()
const vuetifyTheme = useTheme()

let queuePoller: ReturnType<typeof setInterval> | null = null

// Global audio element (declared in template so browser sees it during parse)
const audioEl = ref<HTMLAudioElement | null>(null)
provide('audioEl', audioEl)

// 浏览器 autoplay 策略要求首次播放必须有用户手势触发，否则 .play() 会被静默拒绝。
// 我们尝试自动播一次，被拒就显示一个全屏覆盖层等用户点。
const needsTapToPlay = ref(false)

function initAudio() {
  if (!audioEl.value) return
  audioEl.value.src = getStreamUrl()
  audioEl.value.load()
  requestAnimationFrame(() => {
    const p = audioEl.value?.play()
    if (p && typeof p.then === 'function') {
      p.catch(() => { needsTapToPlay.value = true })
    }
  })
}

function startPlaybackFromGesture() {
  if (!audioEl.value) return
  audioEl.value.play().then(() => {
    needsTapToPlay.value = false
  }).catch(() => {
    // 仍然失败：保留按钮，让用户重试
  })
}

function preferredTheme(): 'light' | 'dark' {
  const selected = THEMES[store.themeIdx]
  if (selected === 'dark' || selected === 'light') return selected
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function syncTheme() {
  vuetifyTheme.global.name.value = preferredTheme()
}

async function init() {
  // Start audio immediately in parallel — don't block on API calls
  initAudio()

  await loadStationInfo()
  connectWebSocket()
  await loadDeviceUser()
  const pollers = startPollers()
  queuePoller = pollers.queuePoller
}

onMounted(init)

const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
const onSystemThemeChange = () => {
  if (THEMES[store.themeIdx] === 'auto') syncTheme()
}

watch(() => store.themeIdx, syncTheme, { immediate: true })
mediaQuery.addEventListener('change', onSystemThemeChange)

onUnmounted(() => {
  mediaQuery.removeEventListener('change', onSystemThemeChange)
  if (queuePoller) stopPollers(queuePoller)
  if (getWs()) getWs()!.close()
  if (audioEl.value) {
    audioEl.value.pause()
    audioEl.value.src = ''
  }
})

const navItems = [
  { name: 'now-playing', path: '/', label: '正在播放', icon: 'mdi-play-circle' },
  { name: 'library', path: '/library', label: '曲库', icon: 'mdi-music-box' },
  { name: 'up-next', path: '/up-next', label: '待播清单', icon: 'mdi-playlist-music' },
  { name: 'settings', path: '/settings', label: '设置', icon: 'mdi-cog' },
]

const currentRouteName = computed(() => route.name as string)

const isAdmin = computed(() => store.deviceUser?.role === 'admin')

const showMiniPlayer = computed(() => {
  return currentRouteName.value !== 'now-playing' && store.playbackState.song_id > 0
})

const drawerOpen = computed({
  get: () => store.isDesktop,
  set: () => {}
})

watch(() => store.showSnackbar, (val) => {
  if (!val) {
    setTimeout(() => {
      store.snackbarText = ''
    }, 200)
  }
})
</script>

<template>
  <v-app class="am-app">
    <!-- Global audio element — declared in template so browser parses autoplay early -->
    <audio
      ref="audioEl"
      autoplay
      preload="auto"
      crossorigin="anonymous"
      class="visually-hidden"
    />

    <!-- Desktop: Side Navigation -->
    <v-navigation-drawer
      v-if="store.isDesktop"
      v-model="drawerOpen"
      permanent
      rail
      rail-width="80"
      color="surface"
      class="am-drawer"
    >
      <div class="d-flex flex-column h-100 py-4">
        <div class="text-center mb-6">
          <v-icon color="primary" size="36">mdi-music-circle</v-icon>
        </div>

        <v-list nav density="compact" class="pa-0">
          <v-list-item
            v-for="item in navItems"
            :key="item.name"
            :to="item.path"
            :active="currentRouteName === item.name"
            rounded="lg"
            class="mb-2 mx-2 justify-center"
            min-height="56"
          >
            <template #prepend>
              <v-icon :icon="item.icon" size="24" />
            </template>
            <v-tooltip
              :text="item.label"
              location="right"
              activator="parent"
            />
          </v-list-item>

          <v-list-item
            v-if="isAdmin"
            to="/admin"
            :active="currentRouteName === 'admin'"
            rounded="lg"
            class="mb-2 mx-2 justify-center"
            min-height="56"
          >
            <template #prepend>
              <v-icon icon="mdi-shield-account" size="24" />
            </template>
            <v-tooltip
              text="管理"
              location="right"
              activator="parent"
            />
          </v-list-item>
        </v-list>

        <v-spacer />

        <div class="text-center">
          <v-btn
            icon
            variant="text"
            size="small"
            color="medium-emphasis"
            @click="$router.push('/settings')"
          >
            <v-icon>mdi-account-circle</v-icon>
          </v-btn>
        </div>
      </div>
    </v-navigation-drawer>

    <!-- Main Content -->
    <v-main class="am-main" :class="{ 'pb-nav': !store.isDesktop }">
      <div class="am-content-wrapper">
        <router-view />
      </div>

      <!-- Mini Player (above bottom nav on mobile, above footer on desktop) -->
      <MiniPlayer v-if="showMiniPlayer" />
    </v-main>

    <!-- Mobile: Bottom Navigation -->
    <v-bottom-navigation
      v-if="!store.isDesktop"
      v-model="currentRouteName"
      color="primary"
      grow
      elevation="8"
      class="am-bottom-nav"
    >
      <v-btn
        v-for="item in navItems"
        :key="item.name"
        :value="item.name"
        :to="item.path"
        variant="text"
      >
        <v-icon>{{ item.icon }}</v-icon>
        <span class="text-caption">{{ item.label }}</span>
      </v-btn>
    </v-bottom-navigation>

    <!-- 浏览器 autoplay 拦截兜底：要点一下才能开始播 -->
    <div
      v-if="needsTapToPlay"
      class="am-tap-overlay"
      @click="startPlaybackFromGesture"
    >
      <div class="am-tap-card">
        <v-icon size="56" color="primary">mdi-play-circle</v-icon>
        <div class="text-h6 mt-3">点击开始收听</div>
        <div class="text-caption text-medium-emphasis mt-1">
          浏览器需要您的一次点击才能开始播放
        </div>
      </div>
    </div>

    <!-- Global Snackbar (replaces ToastContainer) -->
    <v-snackbar
      v-model="store.showSnackbar"
      :color="store.snackbarColor"
      :timeout="3000"
      location="top"
      rounded="lg"
      elevation="16"
    >
      {{ store.snackbarText }}
      <template #actions>
        <v-btn variant="text" color="white" @click="store.showSnackbar = false">
          关闭
        </v-btn>
      </template>
    </v-snackbar>
  </v-app>
</template>

<style scoped>
.am-app {
  font-family: var(--font-display);
}

.am-drawer {
  border-right: 1px solid var(--am-divider);
}

.am-drawer :deep(.v-list-item) {
  display: flex;
  align-items: center;
  justify-content: center;
}

.am-drawer :deep(.v-list-item__prepend) {
  width: 100%;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.am-drawer :deep(.v-list-item__spacer),
.am-drawer :deep(.v-list-item__content) {
  display: none;
}

.am-main {
  background: var(--am-bg);
  transition: padding 0.3s ease;
}

.am-content-wrapper {
  max-width: 1200px;
  margin: 0 auto;
  padding: 16px;
  min-height: 100%;
}

.pb-nav {
  padding-bottom: 128px !important;
}

.am-bottom-nav {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 100;
  background: var(--am-surface) !important;
  border-top: 1px solid var(--am-divider);
}

.am-bottom-nav :deep(.v-btn__content) {
  align-items: center;
  justify-content: center;
  gap: 2px;
  line-height: 1.1;
}

/* Desktop content padding for rail drawer */
@media (min-width: 960px) {
  .am-content-wrapper {
    padding: 24px 32px;
  }
}

/* Tap-to-play overlay (shown when browser blocks autoplay) */
.am-tap-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  animation: am-fade-in 0.2s ease;
}

.am-tap-card {
  background: var(--am-surface);
  border-radius: 16px;
  padding: 32px 40px;
  text-align: center;
  box-shadow: var(--am-shadow-16);
  max-width: 320px;
}

@keyframes am-fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}
</style>
