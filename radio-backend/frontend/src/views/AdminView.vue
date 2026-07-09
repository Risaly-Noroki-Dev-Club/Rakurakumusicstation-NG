<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { store } from '../store'
import AdminUsers from '../components/admin/AdminUsers.vue'
import AdminSongs from '../components/admin/AdminSongs.vue'
import AdminUpload from '../components/admin/AdminUpload.vue'
import AdminDownload from '../components/admin/AdminDownload.vue'
import AdminNcm from '../components/admin/AdminNcm.vue'
import AdminSettings from '../components/admin/AdminSettings.vue'
import AdminStats from '../components/admin/AdminStats.vue'
import LtPageShell from '../components/lt/LtPageShell.vue'

const route = useRoute()
const router = useRouter()

const subtab = computed(() => (route.params.subtab as string) || 'users')

const tabs = [
  { name: 'users', label: '用户管理', icon: 'mdi-account-group-outline' },
  { name: 'songs', label: '歌曲管理', icon: 'mdi-music-note-outline' },
  { name: 'upload', label: '上传', icon: 'mdi-cloud-upload-outline' },
  { name: 'download', label: '下载', icon: 'mdi-download-outline' },
  { name: 'ncm', label: '网易云', icon: 'mdi-music-circle-outline' },
  { name: 'settings', label: '设置', icon: 'mdi-cog-outline' },
  { name: 'stats', label: '统计', icon: 'mdi-chart-bar' },
]

function switchTab(name: string) {
  router.push('/admin/' + name)
}
</script>

<template>
  <!-- 桌面端：LT 风格 -->
  <LtPageShell v-if="store.isDesktop" title="Admin Panel" subtitle="管理电台的内容和设置">
    <div class="lt-admin-layout">
      <!-- 左侧 tab 栏 -->
      <div class="lt-admin-tabs">
        <button
          v-for="t in tabs"
          :key="t.name"
          :class="['lt-admin-tab', { active: subtab === t.name }]"
          @click="switchTab(t.name)"
        >
          <v-icon :icon="t.icon" size="18" />
          <span>{{ t.label }}</span>
        </button>
      </div>

      <!-- 内容区 -->
      <div class="lt-admin-content">
        <div v-show="subtab === 'users'"><AdminUsers /></div>
        <div v-show="subtab === 'songs'"><AdminSongs /></div>
        <div v-show="subtab === 'upload'"><AdminUpload /></div>
        <div v-show="subtab === 'download'"><AdminDownload /></div>
        <div v-show="subtab === 'ncm'"><AdminNcm /></div>
        <div v-show="subtab === 'settings'"><AdminSettings /></div>
        <div v-show="subtab === 'stats'"><AdminStats /></div>
      </div>
    </div>
  </LtPageShell>

  <!-- 移动端：原有布局 -->
  <div v-else class="am-admin">
    <!-- Page Header -->
    <div class="am-page-header mb-5">
      <h1 class="text-h5 font-weight-bold">管理后台</h1>
      <p class="text-body-2 text-medium-emphasis mt-1">管理电台的内容和设置</p>
    </div>

    <v-card elevation="0" class="am-card">
      <v-tabs
        v-model="subtab"
        color="primary"
        grow
        show-arrows
        density="comfortable"
      >
        <v-tab
          v-for="t in tabs"
          :key="t.name"
          :value="t.name"
          @click="switchTab(t.name)"
        >
          <v-icon size="18" class="mr-1">{{ t.icon }}</v-icon>
          {{ t.label }}
        </v-tab>
      </v-tabs>

      <v-card-text class="pa-5">
        <div v-show="subtab === 'users'"><AdminUsers /></div>
        <div v-show="subtab === 'songs'"><AdminSongs /></div>
        <div v-show="subtab === 'upload'"><AdminUpload /></div>
        <div v-show="subtab === 'download'"><AdminDownload /></div>
        <div v-show="subtab === 'ncm'"><AdminNcm /></div>
        <div v-show="subtab === 'settings'"><AdminSettings /></div>
        <div v-show="subtab === 'stats'"><AdminStats /></div>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>
/* ─── LT 桌面端样式 ─── */
.lt-admin-layout {
  display: flex;
  gap: 16px;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.lt-admin-tabs {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex-shrink: 0;
  /* 规则3: 响应式宽度 */
  width: clamp(120px, 18vw, 160px);
  min-width: 0;
}

.lt-admin-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: var(--lt-text-secondary);
  /* 规则1: 响应式缩小字号 */
  font-size: clamp(11px, 1.5vw, 13px);
  font-weight: 500;
  cursor: pointer;
  border-radius: var(--lt-radius-sm);
  transition: background 0.15s, color 0.15s;
  font-family: var(--lt-font-sans);
  text-align: left;
  /* 规则2: 省略号截断 */
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.lt-admin-tab:hover { background: var(--lt-btn-bg); color: var(--lt-text-primary); }

.lt-admin-tab.active {
  background: var(--lt-card-bg);
  color: var(--lt-text-primary);
  font-weight: 600;
  box-shadow: var(--lt-shadow-subtle);
}

.lt-admin-content {
  flex: 1;
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-md);
  padding: 20px;
  overflow-y: auto;
  box-shadow: var(--lt-shadow-subtle);
  min-width: 0;
}

/* ─── 移动端原有样式 ─── */
.am-admin {
  padding-bottom: 16px;
  animation: slideUp 0.5s var(--am-ease-emphasized);
}

.am-page-header {
  animation: slideUp 0.4s var(--am-ease-emphasized);
}

.am-card {
  overflow: hidden;
}

.am-card:hover {
  border-color: transparent;
}

@keyframes slideUp {
  from { transform: translateY(16px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
