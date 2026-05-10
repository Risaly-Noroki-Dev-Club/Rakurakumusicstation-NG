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
  { name: 'users', label: '用户管理', icon: 'mdi-account-group' },
  { name: 'songs', label: '歌曲管理', icon: 'mdi-music-note' },
  { name: 'upload', label: '上传', icon: 'mdi-cloud-upload' },
  { name: 'download', label: '下载', icon: 'mdi-download' },
  { name: 'ncm', label: '网易云', icon: 'mdi-music-circle' },
  { name: 'settings', label: '设置', icon: 'mdi-cog' },
  { name: 'stats', label: '统计', icon: 'mdi-chart-bar' },
]

function switchTab(name: string) {
  router.push('/admin/' + name)
}
</script>

<template>
  <div class="am-admin">
    <v-card elevation="1">
      <v-tabs
        v-model="subtab"
        color="primary"
        grow
        show-arrows
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

      <v-card-text class="pa-4">
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
}
</style>
