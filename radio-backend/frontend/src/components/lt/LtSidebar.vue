<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { store, THEMES, applyTheme } from '../../store'

const route = useRoute()
const router = useRouter()

const navItems = [
  { name: 'now-playing', label: 'Now Listening', path: '/' },
  { name: 'library', label: 'My Library', path: '/library' },
  { name: 'up-next', label: 'Play Queue', path: '/up-next' },
  { name: 'settings', label: 'Account', path: '/settings' },
]

const isAdmin = computed(() => store.deviceUser?.role === 'admin')
const currentName = computed(() => route.name as string)
const stationLabel = computed(() => store.stationName || 'Rakuraku Music Station')

const userName = computed(() => store.deviceUser?.display_name || 'Guest')
const userInitial = computed(() => userName.value.charAt(0).toUpperCase() || '?')

// 头像渐变色 — 基于用户名生成稳定色相
function avatarGradient(name: string): string {
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash)
  const h1 = Math.abs(hash) % 360
  const h2 = (h1 + 40) % 360
  return `linear-gradient(135deg, hsl(${h1}, 70%, 60%), hsl(${h2}, 70%, 55%))`
}

function go(path: string) {
  router.push(path)
}

function selectTheme(idx: number) {
  store.themeIdx = idx
  applyTheme()
}
</script>

<template>
  <nav class="lt-sidebar">
    <div class="lt-brand">{{ stationLabel }}</div>

    <div class="lt-nav-items">
      <div
        v-for="item in navItems"
        :key="item.name"
        class="lt-nav-item"
        :class="{ active: currentName === item.name }"
        @click="go(item.path)"
      >
        <span class="lt-nav-dot"></span>
        <span>{{ item.label }}</span>
      </div>
      <div
        v-if="isAdmin"
        class="lt-nav-item"
        :class="{ active: currentName === 'admin' }"
        @click="go('/admin/users')"
      >
        <span class="lt-nav-dot"></span>
        <span>Admin</span>
      </div>
    </div>

    <!-- 主题切换 -->
    <div class="lt-theme-switcher">
      <button
        v-for="(t, idx) in THEMES"
        :key="t"
        class="lt-theme-btn"
        :class="{ active: store.themeIdx === idx }"
        :title="t === 'auto' ? '跟随系统' : t === 'dark' ? '深色模式' : '浅色模式'"
        @click="selectTheme(idx)"
      >
        <!-- auto -->
        <svg v-if="t === 'auto'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <rect x="3" y="3" width="7" height="7" rx="1" />
          <path d="M14 15h7M14 19h7M17 15v8" />
          <circle cx="6.5" cy="6.5" r="1.5" fill="currentColor" />
        </svg>
        <!-- sun -->
        <svg v-else-if="t === 'light'" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
          <circle cx="12" cy="12" r="5" />
          <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none" />
        </svg>
        <!-- moon -->
        <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      </button>
    </div>

    <div class="lt-nav-spacer"></div>

    <div class="lt-nav-user" @click="go('/settings')">
      <div class="lt-user-avatar" :style="{ background: avatarGradient(userName) }">
        {{ userInitial }}
      </div>
      <span class="lt-user-name">{{ userName }}</span>
    </div>
  </nav>
</template>

<style scoped>
.lt-sidebar {
  width: 220px;
  min-width: 220px;
  background: var(--lt-sidebar-bg);
  display: flex;
  flex-direction: column;
  padding: 24px 16px 16px;
  flex-shrink: 0;
}

.lt-brand {
  font-family: var(--lt-font-serif);
  font-size: 20px;
  font-weight: 700;
  color: var(--lt-text-primary);
  padding: 0 16px 20px;
  letter-spacing: -0.3px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-nav-items {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.lt-nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 16px;
  border-radius: 18px;
  font-family: var(--lt-font-serif);
  font-size: 14px;
  font-weight: 600;
  color: var(--lt-text-secondary);
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
  user-select: none;
}

.lt-nav-item:hover {
  background: rgba(212, 201, 168, 0.4);
}

.lt-nav-item.active {
  background: var(--lt-selected-bg);
  color: var(--lt-text-primary);
}

.lt-nav-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: transparent;
  flex-shrink: 0;
  transition: background 0.2s ease;
}

.lt-nav-item.active .lt-nav-dot {
  background: var(--lt-accent);
}

.lt-theme-switcher {
  display: flex;
  gap: 4px;
  padding: 8px 16px;
  background: var(--lt-card-bg);
  border-radius: 12px;
  margin-top: 16px;
  align-self: stretch;
}

.lt-theme-btn {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: 1px solid var(--lt-divider);
  background: transparent;
  color: var(--lt-text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s ease;
  padding: 0;
}

.lt-theme-btn:hover {
  background: var(--lt-selected-bg);
  color: var(--lt-text-primary);
}

.lt-theme-btn.active {
  background: var(--lt-accent-soft);
  border-color: var(--lt-accent);
  color: var(--lt-accent-text);
}

.lt-nav-spacer {
  flex: 1;
}

.lt-nav-user {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: 12px;
  cursor: pointer;
  transition: background 0.15s ease;
}

.lt-nav-user:hover {
  background: rgba(212, 201, 168, 0.3);
}

.lt-user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  color: #fff;
  flex-shrink: 0;
}

.lt-user-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--lt-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
