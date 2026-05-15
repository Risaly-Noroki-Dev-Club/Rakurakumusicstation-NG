<script setup lang="ts">
import { store, toast } from '../../store'
import { apiUrl } from '../../api'

async function adminRescanSongs() {
  toast('正在重新扫描...', 'info')
  try {
    await fetch(apiUrl('/api/admin/rescan-songs'), { method: 'POST' })
    toast('扫描完成', 'success')
    loadSongs()
  } catch { toast('扫描失败', 'error') }
}

async function adminPlayNext() {
  try {
    await fetch(apiUrl('/api/admin/playlist/next'), { method: 'POST' })
    toast('已切到下一首', 'success')
  } catch { toast('请求失败', 'error') }
}

async function adminPlayPrev() {
  try {
    await fetch(apiUrl('/api/admin/playlist/prev'), { method: 'POST' })
    toast('已切到上一首', 'success')
  } catch { toast('请求失败', 'error') }
}

async function adminDeleteSong(id: number) {
  if (!confirm('确定删除此歌曲？')) return
  try {
    const r = await fetch(apiUrl('/api/admin/songs/' + id), { method: 'DELETE' })
    const d = await r.json()
    toast(d.success ? '已删除' : (d.error || '删除失败'), d.success ? 'success' : 'error')
    if (d.success) loadSongs()
  } catch { toast('删除失败', 'error') }
}

async function loadSongs() {
  try {
    const r = await fetch(apiUrl('/api/admin/songs'))
    const d = await r.json()
    if (d.success) store.adminSongs = d.data || []
  } catch { /* ignore */ }
}

loadSongs()

const headers = [
  { title: '标题', key: 'title' },
  { title: '艺术家', key: 'artist' },
  { title: '操作', key: 'actions', sortable: false, width: '80' },
]
</script>

<template>
  <div>
    <div class="d-flex flex-wrap gap-2 mb-4">
      <v-btn color="info" prepend-icon="mdi-refresh" size="small" @click="adminRescanSongs">
        重新扫描
      </v-btn>
      <v-btn color="primary" prepend-icon="mdi-skip-next" size="small" @click="adminPlayNext">
        下一首
      </v-btn>
      <v-btn color="secondary" prepend-icon="mdi-skip-previous" size="small" @click="adminPlayPrev">
        上一首
      </v-btn>
    </div>

    <v-data-table
      :headers="headers"
      :items="store.adminSongs"
      density="compact"
      class="elevation-0"
    >
      <template #item.actions="{ item }">
        <v-btn
          icon
          variant="text"
          color="error"
          size="small"
          @click="adminDeleteSong(item.id)"
        >
          <v-icon size="18">mdi-delete</v-icon>
        </v-btn>
      </template>
    </v-data-table>
  </div>
</template>

<style scoped>
.gap-2 {
  gap: 8px;
}
</style>
