<script setup lang="ts">
import { onMounted, onUnmounted, computed, watch, provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { store } from './store'
import {
  loadStationInfo, loadDeviceUser,
  connectWebSocket, startPollers, stopPollers, getWs
} from './api'
import { useLiveAudio } from './app/useLiveAudio'
import { useThemeSync } from './app/useThemeSync'
import { useListenTogetherLayout } from './composables/useListenTogetherLayout'
import MiniPlayer from './components/MiniPlayer.vue'

const route = useRoute()
const router = useRouter()
const vuetifyTheme = useTheme()

let queuePoller: ReturnType<typeof setInterval> | null = null

const {
  audioEl,
  needsTapToPlay,
  initAudio,
  restartLiveStream,
  startPlaybackFromGesture,
  cleanupAudio,
} = useLiveAudio()
provide('audioEl', audioEl)
useThemeSync(vuetifyTheme)
const { isActive: ltActive } = useListenTogetherLayout()

async function init() {
  initAudio()
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
  cleanupAudio()
})

const navItems = [
  { name: 'now-playing', path: '/', label: '播放', icon: 'mdi-play-circle-outline' },
  { name: 'library', path: '/library', label: '曲库', icon: 'mdi-bookshelf' },
  { name: 'up-next', path: '/up-next', label: '队列', icon: 'mdi-format-list-numbered' },
  { name: 'settings', path: '/settings', label: '设置', icon: 'mdi-cog-outline' },
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

const stationLabel = computed(() => store.stationName || 'Rakuraku Music Station')

watch(() => store.showSnackbar, (val) => {
  if (!val) {
    setTimeout(() => {
      store.snackbarText = ''
    }, 200)
  }
})

watch(
  () => `${store.playbackState.song_id}:${store.playbackState.title}:${store.playbackState.artist}`,
  (_current, previous) => {
    if (previous) restartLiveStream()
  }
)
</script>

<template>
  <v-app class="am-app">
    <!-- Global audio element -->
    <audio
      ref="audioEl"
      autoplay
      preload="auto"
      crossorigin="anonymous"
      class="visually-hidden"
    />

    <!-- Desktop: Side Navigation (隐藏于 Listen Together 三栏布局) -->
    <v-navigation-drawer
      v-if="store.isDesktop && !ltActive"
      v-model="drawerOpen"
      permanent
      rail
      rail-width="88"
      color="transparent"
      class="am-drawer"
    >
      <div class="d-flex flex-column h-100 py-5">
        <!-- Logo -->
        <div class="text-center mb-8 px-2">
          <div class="am-logo">
            <v-icon color="primary" size="32">mdi-radio-tower</v-icon>
          </div>
          <div class="am-logo-label text-caption font-weight-bold mt-2 text-truncate">
            {{ stationLabel }}
          </div>
        </div>

        <!-- Nav items -->
        <v-list nav density="compact" class="pa-0">
          <v-list-item
            v-for="item in navItems"
            :key="item.name"
            :to="item.path"
            :active="currentRouteName === item.name"
            rounded="xl"
            class="mb-1 mx-3 justify-center am-nav-item"
            min-height="52"
          >
            <template #prepend>
              <v-icon :icon="item.icon" size="22" />
            </template>
            <v-tooltip
              :text="item.label"
              location="right"
              activator="parent"
            />
          </v-list-item>

          <!-- Admin -->
          <v-list-item
            v-if="isAdmin"
            to="/admin"
            :active="currentRouteName === 'admin'"
            rounded="xl"
            class="mb-1 mx-3 justify-center am-nav-item"
            min-height="52"
          >
            <template #prepend>
              <v-icon icon="mdi-shield-account-outline" size="22" />
            </template>
            <v-tooltip
              text="管理"
              location="right"
              activator="parent"
            />
          </v-list-item>
        </v-list>

        <v-spacer />

        <!-- Bottom: user avatar -->
        <div class="text-center px-3">
          <v-btn
            icon
            variant="tonal"
            size="small"
            color="primary"
            @click="$router.push('/settings')"
            class="am-avatar-btn"
          >
            <v-icon size="20">mdi-account-circle-outline</v-icon>
          </v-btn>
        </div>
      </div>
    </v-navigation-drawer>

    <!-- Main Content -->
    <v-main class="am-main" :class="{ 'pb-nav': !store.isDesktop, 'lt-fullscreen': ltActive }">
      <div class="am-content-wrapper" :class="{ 'lt-content': ltActive }">
        <router-view />
      </div>

      <!-- Mini Player -->
      <MiniPlayer v-if="showMiniPlayer" />
    </v-main>

    <!-- Mobile: Bottom Navigation -->
    <v-bottom-navigation
      v-if="!store.isDesktop"
      v-model="currentRouteName"
      color="primary"
      grow
      elevation="0"
      class="am-bottom-nav"
      height="64"
    >
      <v-btn
        v-for="item in navItems"
        :key="item.name"
        :value="item.name"
        :to="item.path"
        variant="text"
        class="am-mobile-btn"
      >
        <v-icon>{{ item.icon }}</v-icon>
        <span class="text-caption font-weight-semibold">{{ item.label }}</span>
      </v-btn>
    </v-bottom-navigation>

    <!-- Tap-to-play overlay -->
    <div
      v-if="needsTapToPlay"
      class="am-tap-overlay"
      @click="startPlaybackFromGesture"
    >
      <div class="am-tap-card">
        <div class="am-tap-icon">
          <v-icon size="52" color="primary">mdi-play-circle</v-icon>
        </div>
        <div class="text-h6 font-weight-bold mt-4">点击开始收听</div>
        <div class="text-body-2 text-medium-emphasis mt-2">
          浏览器需要您的一次点击才能开始播放
        </div>
      </div>
    </div>

    <!-- Global Snackbar -->
    <v-snackbar
      v-model="store.showSnackbar"
      :color="store.snackbarColor"
      :timeout="3000"
      location="top"
      rounded="xl"
      elevation="8"
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
  border-right: none !important;
  background: transparent !important;
}

.am-drawer :deep(.v-list-item) {
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.28s var(--am-ease-smooth);
}

.am-drawer :deep(.v-list-item--active) {
  background: var(--am-primary) !important;
  color: white !important;
}

.am-drawer :deep(.v-list-item--active .v-icon) {
  color: white !important;
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

.am-logo {
  width: 48px;
  height: 48px;
  margin: 0 auto;
  border-radius: var(--am-radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--am-surface-2);
  border: 1px solid var(--am-divider);
  transition: all 0.3s var(--am-ease-emphasized);
}

.am-logo:hover {
  background: var(--am-primary);
  border-color: var(--am-primary);
}

.am-logo:hover .v-icon {
  color: white !important;
}

.am-logo-label {
  color: var(--am-text-high);
}

.am-avatar-btn {
  width: 40px;
  height: 40px;
}

.am-main {
  background: var(--am-bg);
  transition: padding 0.3s var(--am-ease-emphasized);
}

/* Listen Together 全屏模式：移除 Vuetify drawer 预留的内边距 */
.am-main.lt-fullscreen {
  padding-left: 0 !important;
  padding-right: 0 !important;
}

.am-content-wrapper {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
  min-height: 100%;
}

.am-content-wrapper.lt-content {
  max-width: none;
  margin: 0;
  padding: 0;
  height: 100%;
}

.pb-nav {
  padding-bottom: 140px !important;
}

.am-bottom-nav {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 100;
  border-top: none !important;
}

.am-bottom-nav :deep(.v-btn__content) {
  align-items: center;
  justify-content: center;
  gap: 3px;
  line-height: 1.1;
}

/* Desktop content padding */
@media (min-width: 960px) {
  .am-content-wrapper {
    padding: 28px 40px;
  }
}

/* Tap-to-play overlay */
.am-tap-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(0, 0, 0, 0.70);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  animation: fadeIn 0.3s var(--am-ease-emphasized);
}

.am-tap-card {
  background: var(--am-surface);
  border-radius: var(--am-radius-xl);
  padding: 40px 48px;
  text-align: center;
  box-shadow: var(--am-shadow-16);
  max-width: 340px;
  border: 1px solid var(--am-divider);
}

.am-tap-icon {
  animation: float 2s ease-in-out infinite;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-6px); }
}
</style>
