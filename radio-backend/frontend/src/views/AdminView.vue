<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AdminUsers from '../components/admin/AdminUsers.vue'
import AdminSongs from '../components/admin/AdminSongs.vue'
import AdminUpload from '../components/admin/AdminUpload.vue'
import AdminDownload from '../components/admin/AdminDownload.vue'
import AdminNcm from '../components/admin/AdminNcm.vue'
import AdminSettings from '../components/admin/AdminSettings.vue'
import AdminStats from '../components/admin/AdminStats.vue'

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
  <div class="am-admin">
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
