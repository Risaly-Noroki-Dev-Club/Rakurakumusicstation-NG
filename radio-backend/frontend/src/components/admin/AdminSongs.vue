<script setup lang="ts">
import { store, toast } from '../../store'
import { getBackendUrl } from '../../api'

function authHeaders() {
  const h: Record<string, string> = {}; if (store.token) h.Authorization = "Bearer " + store.token; return h
}

async function adminRescanSongs() {
  toast('正在重新扫描...', 'info')
  try {
    await fetch(getBackendUrl() + '/api/admin/rescan-songs', { method: 'POST', headers: authHeaders() })
    toast('扫描完成', 'success')
    loadSongs()
  } catch { toast('扫描失败', 'error') }
}

async function adminPlayNext() {
  try {
    await fetch(getBackendUrl() + '/api/admin/playlist/next', { method: 'POST', headers: authHeaders() })
    toast('已切到下一首', 'success')
  } catch { toast('请求失败', 'error') }
}

async function adminPlayPrev() {
  try {
    await fetch(getBackendUrl() + '/api/admin/playlist/prev', { method: 'POST', headers: authHeaders() })
    toast('已切到上一首', 'success')
  } catch { toast('请求失败', 'error') }
}

async function adminDeleteSong(id: number) {
  if (!confirm('确定删除此歌曲？')) return
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/songs/' + id, { method: 'DELETE', headers: authHeaders() })
    const d = await r.json()
    toast(d.success ? '已删除' : (d.error || '删除失败'), d.success ? 'success' : 'error')
    if (d.success) loadSongs()
  } catch { toast('删除失败', 'error') }
}

async function loadSongs() {
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/songs', { headers: authHeaders() })
    const d = await r.json()
    if (d.success) store.adminSongs = d.data || []
  } catch { /* ignore */ }
}

const emit = defineEmits<{ load: [] }>()
loadSongs()
</script>

<template>
  <div class="card admin-panel">
    <h2>🎵 歌曲管理</h2>
    <div style="display:flex;gap:8px;margin-bottom:12px;flex-wrap:wrap">
      <button class="btn btn-info btn-small" @click="adminRescanSongs">🔄 重新扫描</button>
      <button class="btn btn-primary btn-small" @click="adminPlayNext">⏭ 下一首</button>
      <button class="btn btn-secondary btn-small" @click="adminPlayPrev">⏮ 上一首</button>
    </div>
    <div style="max-height:500px;overflow-y:auto">
      <table class="song-table">
        <thead><tr><th>标题</th><th>艺术家</th><th>操作</th></tr></thead>
        <tbody>
          <tr v-if="store.adminSongs.length === 0">
            <td colspan="3" style="text-align:center;color:var(--text-muted);padding:20px">点击重新扫描加载歌曲</td>
          </tr>
          <tr v-for="s in store.adminSongs" :key="s.id">
            <td class="song-title" :title="s.title">{{ s.title }}</td>
            <td style="color:var(--text-muted)">{{ s.artist || '-' }}</td>
            <td class="actions">
              <button class="btn btn-danger btn-small" @click="adminDeleteSong(s.id)">🗑️</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
