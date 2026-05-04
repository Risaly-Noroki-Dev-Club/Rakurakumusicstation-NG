<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { store, toast } from '../store'
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

const subtabs = [
  { name: 'users', label: '👥 用户管理' },
  { name: 'songs', label: '🎵 歌曲管理' },
  { name: 'upload', label: '📤 上传' },
  { name: 'download', label: '⬇️ 下载' },
  { name: 'ncm', label: '🎵 网易云' },
  { name: 'settings', label: '⚙️ 设置' },
  { name: 'stats', label: '📊 统计' },
]

function switchSubtab(name: string) {
  router.push('/admin/' + name)
}
</script>

<template>
  <div>
    <div class="admin-subtabs">
      <button v-for="s in subtabs" :key="s.name"
              :class="{ subtab: true, active: subtab === s.name }"
              @click="switchSubtab(s.name)">{{ s.label }}</button>
    </div>

    <div v-show="subtab === 'users'"><AdminUsers /></div>
    <div v-show="subtab === 'songs'"><AdminSongs /></div>
    <div v-show="subtab === 'upload'"><AdminUpload /></div>
    <div v-show="subtab === 'download'"><AdminDownload /></div>
    <div v-show="subtab === 'ncm'"><AdminNcm /></div>
    <div v-show="subtab === 'settings'"><AdminSettings /></div>
    <div v-show="subtab === 'stats'"><AdminStats /></div>
  </div>
</template>
