<script setup lang="ts">
import { computed, watch, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { store, toast } from '../store'
import { getBackendUrl } from '../api'

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

function authHeaders(): Record<string, string> {
  const h: Record<string, string> = {}
  if (store.token) h['Authorization'] = 'Bearer ' + store.token
  return h
}

async function loadData() {
  switch (subtab.value) {
  case 'users': await loadAdminUsersAndLogs(); break
  case 'songs': await loadAdminSongs(); break
  case 'stats': await loadAdminStats(); break
  case 'settings': await loadSettings(); break
  case 'ncm': await loadNcmStatus(); break
  }
}

watch(subtab, () => loadData(), { immediate: true })

// Ping download status on mount if a download is running
let downloadPoller: ReturnType<typeof setInterval> | null = null
function checkDownloadOnInit() {
  if (!store.token) return
  fetch(getBackendUrl() + '/api/admin/download/status', { headers: authHeaders() })
    .then(r => r.json())
    .then(data => {
      if (data.success && data.data && data.data.running) {
        store.downloadRunning = true
        store.downloadLog = data.data.log || ''
        store.downloadStatusMsg = '下载中，请稍候...'
        store.downloadStatusType = 'info'
        pollDownloadStatus()
      }
    }).catch(() => {})
}

onMounted(checkDownloadOnInit)
onUnmounted(() => { if (downloadPoller) clearInterval(downloadPoller) })

// ─── USERS ───
async function loadAdminUsersAndLogs() {
  if (!store.currentUser || store.currentUser.role !== 'admin') return
  try {
    const [usersRes, logsRes] = await Promise.all([
      fetch(getBackendUrl() + '/api/admin/users', { headers: authHeaders() }),
      fetch(getBackendUrl() + '/api/admin/logs', { headers: authHeaders() }),
    ])
    store.users = ((await usersRes.json()).data || [])
    store.adminLogs = ((await logsRes.json()).data || [])
  } catch { /* ignore */ }
}

async function adminBan(id: number) {
  try {
    await fetch(getBackendUrl() + '/api/admin/users/' + id + '/ban', {
      method: 'POST', headers: authHeaders()
    })
    loadAdminUsersAndLogs()
  } catch { /* ignore */ }
}

async function adminUnban(id: number) {
  try {
    await fetch(getBackendUrl() + '/api/admin/users/' + id + '/unban', {
      method: 'POST', headers: authHeaders()
    })
    loadAdminUsersAndLogs()
  } catch { /* ignore */ }
}

async function adminPromote(id: number) {
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/users/' + id + '/role', {
      method: 'PUT',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'admin' })
    })
    const data = await res.json()
    if (data.success) { toast(data.data, 'success'); loadAdminUsersAndLogs() }
    else toast(data.error || '提权失败', 'error')
  } catch { toast('操作失败', 'error') }
}

async function adminDemote(id: number) {
  if (!confirm('确定要撤销该用户的管理员权限吗？')) return
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/users/' + id + '/role', {
      method: 'PUT',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'user' })
    })
    const data = await res.json()
    if (data.success) { toast(data.data, 'success'); loadAdminUsersAndLogs() }
    else toast(data.error || '降权失败', 'error')
  } catch { toast('操作失败', 'error') }
}

// ─── SONGS ───
async function loadAdminSongs() {
  if (!store.currentUser || store.currentUser.role !== 'admin') return
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/songs', { headers: authHeaders() })
    store.adminSongs = ((await res.json()).data || [])
  } catch { toast('加载歌曲列表失败', 'error') }
}

async function adminDeleteSong(id: number) {
  if (!confirm('确定要删除这首歌曲吗？此操作不可撤销。')) return
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/songs/' + id, {
      method: 'DELETE', headers: authHeaders()
    })
    const data = await res.json()
    if (data.success) { toast(data.data, 'success'); loadAdminSongs() }
    else toast(data.error || '删除失败', 'error')
  } catch { toast('删除失败', 'error') }
}

async function adminRescanSongs() {
  try {
    toast('正在扫描媒体目录...', 'info')
    const res = await fetch(getBackendUrl() + '/api/admin/rescan-songs', {
      method: 'POST', headers: authHeaders()
    })
    const data = await res.json()
    if (data.success) { toast(data.data, 'success'); loadAdminSongs() }
    else toast(data.error || '扫描失败', 'error')
  } catch { toast('扫描失败', 'error') }
}

async function adminPlayNext() {
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/playlist/next', {
      method: 'POST', headers: authHeaders()
    })
    toast((await res.json()).data || '已切到下一首', 'success')
  } catch { toast('操作失败', 'error') }
}

async function adminPlayPrev() {
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/playlist/prev', {
      method: 'POST', headers: authHeaders()
    })
    toast((await res.json()).data || '已切到上一首', 'success')
  } catch { toast('操作失败', 'error') }
}

// ─── STATS ───
async function loadAdminStats() {
  if (!store.currentUser || store.currentUser.role !== 'admin') return
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/stats', { headers: authHeaders() })
    store.adminStats = (await res.json()).data || {}
  } catch { /* ignore */ }
}

// ─── UPLOAD ───
function handleFileSelect(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) {
    store.uploadFile = file
    store.uploadFileName = file.name
    store.uploadStatus = ''
    store.uploadStatusType = ''
  }
}

async function uploadSong() {
  if (!store.uploadFile) {
    store.uploadStatus = '请选择文件'
    store.uploadStatusType = 'error'
    return
  }
  if (store.uploadFile.size > 100 * 1024 * 1024) {
    store.uploadStatus = '文件大小超过 100MB 限制'
    store.uploadStatusType = 'error'
    return
  }
  const formData = new FormData()
  formData.append('file', store.uploadFile)
  store.uploadStatus = '上传中...'
  store.uploadStatusType = 'info'
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/upload', {
      method: 'POST',
      headers: { 'Authorization': 'Bearer ' + (store.token || '') },
      body: formData
    })
    const data = await res.json()
    if (data.success) {
      store.uploadStatus = '✅ ' + data.data
      store.uploadStatusType = 'success'
      store.uploadFile = null
      store.uploadFileName = ''
      const fileInput = document.querySelector('.upload-file-input') as HTMLInputElement | null
      if (fileInput) fileInput.value = ''
      loadAdminSongs()
    } else {
      store.uploadStatus = '❌ ' + (data.error || '上传失败')
      store.uploadStatusType = 'error'
    }
  } catch {
    store.uploadStatus = '❌ 上传失败'
    store.uploadStatusType = 'error'
  }
}

// ─── DOWNLOAD ───
async function startDownload() {
  const playlist = store.downloadPlaylist.trim()
  if (!playlist) {
    store.downloadStatusMsg = '请输入歌单内容'
    store.downloadStatusType = 'error'
    return
  }
  store.downloadRunning = true
  store.downloadLog = ''
  store.downloadStatusMsg = '正在提交任务...'
  store.downloadStatusType = 'info'
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/download', {
      method: 'POST',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify({
        playlist,
        quality: store.downloadQuality,
        format: store.downloadFormat
      })
    })
    const data = await res.json()
    if (data.success) {
      store.downloadStatusMsg = '下载中，请稍候...'
      store.downloadStatusType = 'info'
      pollDownloadStatus()
    } else {
      store.downloadStatusMsg = '❌ ' + (data.error || '启动失败')
      store.downloadStatusType = 'error'
      store.downloadRunning = false
    }
  } catch {
    store.downloadStatusMsg = '❌ 启动失败'
    store.downloadStatusType = 'error'
    store.downloadRunning = false
  }
}

function pollDownloadStatus() {
  if (downloadPoller) clearInterval(downloadPoller)
  downloadPoller = setInterval(async () => {
    try {
      const res = await fetch(getBackendUrl() + '/api/admin/download/status', { headers: authHeaders() })
      if (!res.ok) return
      const data = await res.json()
      if (!data.success) return
      const status = data.data
      if (status.log) store.downloadLog = status.log
      if (!status.running) {
        if (downloadPoller) clearInterval(downloadPoller)
        downloadPoller = null
        store.downloadRunning = false
        store.downloadStatusMsg = '✅ 下载完成'
        store.downloadStatusType = 'success'
      }
    } catch { /* ignore */ }
  }, 2000)
}

// ─── NCM ───
async function loadNcmStatus() {
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/ncm', { headers: authHeaders() })
    if (!res.ok) return
    const d = await res.json()
    if (!d.success) return
    const data = d.data
    if (data.configured) {
      const label = data.method === 'cookie' ? 'Cookie 已配置' : '手机号 ' + (data.phone_hint || '') + ' 已配置'
      store.ncmBadge = '✓ ' + label
      store.ncmBadgeClass = 'ok'
    } else {
      store.ncmBadge = '未配置（游客模式）'
      store.ncmBadgeClass = 'none'
    }
  } catch { /* ignore */ }
}

async function saveNcmSettings() {
  const payload = store.ncmActiveTab === 'cookie'
    ? { cookie: store.ncmCookie.trim(), phone: '', password: '' }
    : { phone: store.ncmPhone.trim(), password: store.ncmPassword, cookie: '' }
  if (store.ncmActiveTab === 'cookie' && !payload.cookie)
    return showNcmResult('请填写 Cookie', 'error')
  if (store.ncmActiveTab === 'phone' && (!payload.phone || !payload.password))
    return showNcmResult('请填写手机号和密码', 'error')
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/ncm', {
      method: 'POST',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    const data = await res.json()
    if (data.success) { showNcmResult('✅ 保存成功', 'success'); loadNcmStatus() }
    else showNcmResult('❌ ' + (data.error || '保存失败'), 'error')
  } catch { showNcmResult('❌ 请求失败', 'error') }
}

async function testNcmLogin() {
  showNcmResult('测试中...', 'info')
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/ncm/test', {
      method: 'POST', headers: authHeaders()
    })
    const data = await res.json()
    if (data.success) {
      const d = data.data
      showNcmResult(
        (d.success ? '✅ ' : '❌ ') + (d.output || (d.success ? '登录成功' : '登录失败')),
        d.success ? 'success' : 'error'
      )
    } else showNcmResult('❌ 请求失败', 'error')
  } catch { showNcmResult('❌ 请求失败', 'error') }
}

function showNcmResult(msg: string, type: string) {
  store.ncmResult = msg
  store.ncmResultType = type
}

// ─── SETTINGS ───
async function loadSettings() {
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/settings', { headers: authHeaders() })
    if (!res.ok) { store.settingsResult = '加载设置失败'; store.settingsResultType = 'error'; return }
    const data = await res.json()
    if (!data.success) { store.settingsResult = data.error || '加载失败'; store.settingsResultType = 'error'; return }
    const s = data.data
    store.settingsStationName = s.station_name || 'Rakuraku Music Station'
    store.settingsSubtitle = s.subtitle || ''
    store.settingsPrimaryColor = s.primary_color || '#764ba2'
    store.settingsSecondaryColor = s.secondary_color || '#667eea'
    store.settingsBgColor = s.bg_color || '#f4f4f9'
    store.settingsAdminPassword = ''
    store.settingsResult = '设置已加载'
    store.settingsResultType = 'info'
  } catch (e: any) {
    store.settingsResult = '加载设置失败: ' + (e && e.message)
    store.settingsResultType = 'error'
  }
}

async function saveSettings() {
  try {
    const settings: Record<string, string> = {
      station_name: store.settingsStationName.trim(),
      subtitle: store.settingsSubtitle.trim(),
      primary_color: store.settingsPrimaryColor,
      secondary_color: store.settingsSecondaryColor,
      bg_color: store.settingsBgColor,
    }
    if (store.settingsAdminPassword.trim()) {
      settings.admin_password = store.settingsAdminPassword.trim()
    }
    const res = await fetch(getBackendUrl() + '/api/admin/settings', {
      method: 'POST',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify(settings)
    })
    const data = await res.json()
    if (data.success) {
      store.settingsResult = data.data || '设置保存成功'
      store.settingsResultType = 'success'
    } else {
      store.settingsResult = '保存失败: ' + (data.error || '')
      store.settingsResultType = 'error'
    }
  } catch (e: any) {
    store.settingsResult = '保存失败: ' + (e && e.message)
    store.settingsResultType = 'error'
  }
}
</script>

<template>
  <div>
    <div class="admin-subtabs">
      <button v-for="s in subtabs" :key="s.name"
              :class="{ subtab: true, active: subtab === s.name }"
              @click="switchSubtab(s.name)">{{ s.label }}</button>
    </div>

    <!-- Users Panel -->
    <div v-show="subtab === 'users'">
      <div class="card admin-panel">
        <h2>👥 用户管理</h2>
        <table class="users-table">
          <thead><tr><th>ID</th><th>用户名</th><th>角色</th><th>状态</th><th>操作</th></tr></thead>
          <tbody>
            <tr v-for="u in store.users" :key="u.id">
              <td>{{ u.id }}</td><td>{{ u.username }}</td>
              <td>
                <span :class="['role-badge', u.role === 'admin' ? 'role-admin' : 'role-user']">
                  {{ u.role === 'admin' ? '管理员' : '用户' }}
                </span>
              </td>
              <td>{{ (u as any).is_banned ? '🔴已封禁' : '🟢正常' }}</td>
              <td>
                <button v-if="u.role !== 'admin'" class="btn btn-primary btn-small" @click="adminPromote(u.id)">⭐ 提权</button>
                <button v-if="u.role === 'admin'" class="btn btn-warning btn-small" @click="adminDemote(u.id)">⬇ 降权</button>
                <button v-if="(u as any).is_banned" class="btn btn-secondary btn-small" @click="adminUnban(u.id)">解封</button>
                <button v-else class="btn btn-danger btn-small" @click="adminBan(u.id)">封禁</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="card admin-panel">
        <h2>📋 操作日志 (最近100条)</h2>
        <div style="max-height:300px;overflow-y:auto;font-size:0.85em">
          <div v-for="(l, idx) in store.adminLogs" :key="idx" style="padding:4px 0;border-bottom:1px solid rgba(0,0,0,0.05)">
            <span style="color:var(--text-muted)">[{{ l.created_at }}]</span>
            {{ l.action }} — {{ l.details }}
          </div>
        </div>
      </div>
    </div>

    <!-- Songs Panel -->
    <div v-show="subtab === 'songs'">
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
    </div>

    <!-- Upload Panel -->
    <div v-show="subtab === 'upload'">
      <div class="card">
        <h2>📤 上传新音乐</h2>
        <div class="upload-section">
          <input type="file" class="upload-file-input" accept=".mp3,.wav,.flac,.ogg,.m4a,.aac" @change="handleFileSelect">
          <button class="btn btn-primary" @click="uploadSong" :disabled="!store.uploadFile">上传文件</button>
        </div>
        <div v-if="store.uploadStatus" :style="{
          marginTop: '12px', padding: '10px 14px', borderRadius: '8px', fontSize: '0.88em', display: 'block',
          background: store.uploadStatusType === 'success' ? 'rgba(40,167,69,0.15)' : store.uploadStatusType === 'error' ? 'rgba(220,53,69,0.15)' : 'rgba(23,162,184,0.15)',
          color: store.uploadStatusType === 'success' ? 'var(--success)' : store.uploadStatusType === 'error' ? 'var(--danger)' : 'var(--info)'
        }" v-text="store.uploadStatus"></div>
      </div>
    </div>

    <!-- Download Panel -->
    <div v-show="subtab === 'download'">
      <div class="card">
        <h2>⬇️ 批量下载歌单</h2>
        <textarea v-model="store.downloadPlaylist" rows="8" class="download-input"
          placeholder="每行一首，格式：艺术家 - 歌名&#10;例：&#10;toe - tremolo+delay&#10;Whale Fall - True Places&#10;rinri - 君の世界は透明なんだね"></textarea>
        <div class="download-options">
          <div class="form-group" style="margin-bottom:0">
            <label>音质：</label>
            <select v-model="store.downloadQuality">
              <option value="exhigh">超高音质 (320k)</option>
              <option value="lossless">无损</option>
              <option value="high">高音质 (192k)</option>
              <option value="standard">标准 (128k)</option>
            </select>
          </div>
          <div class="form-group" style="margin-bottom:0">
            <label>备用格式：</label>
            <select v-model="store.downloadFormat">
              <option value="mp3">MP3</option>
              <option value="flac">FLAC</option>
              <option value="m4a">M4A</option>
              <option value="opus">Opus</option>
            </select>
          </div>
          <button class="btn btn-primary" @click="startDownload" :disabled="store.downloadRunning">开始下载</button>
        </div>
        <div v-if="store.downloadStatusMsg" :style="{
          marginTop: '12px', padding: '10px 14px', borderRadius: '8px', fontSize: '0.88em', display: 'block',
          background: store.downloadStatusType === 'success' ? 'rgba(40,167,69,0.15)' : store.downloadStatusType === 'error' ? 'rgba(220,53,69,0.15)' : 'rgba(23,162,184,0.15)',
          color: store.downloadStatusType === 'success' ? 'var(--success)' : store.downloadStatusType === 'error' ? 'var(--danger)' : 'var(--info)'
        }" v-text="store.downloadStatusMsg"></div>
        <div v-if="store.downloadLog" class="download-log" style="display:block" v-text="store.downloadLog"
             :style="{ background: 'var(--card)', color: 'var(--text)', padding: '14px', borderRadius: '8px', fontSize: '0.82em', maxHeight: '280px', overflowY: 'auto', whiteSpace: 'pre-wrap', wordBreak: 'break-all', fontFamily: 'monospace', marginTop: '12px' }"></div>
      </div>
    </div>

    <!-- NCM Panel -->
    <div v-show="subtab === 'ncm'">
      <div class="card">
        <h2>🎵 网易云账号</h2>
        <p style="color:var(--text-muted);font-size:0.85em;margin-bottom:12px">登录后可下载 VIP 歌曲。Cookie 方式更稳定，手机号方式可能触发验证码。</p>
        <div :class="['ncm-badge', store.ncmBadgeClass]" v-text="store.ncmBadge"></div>
        <div class="ncm-tabs">
          <button :class="['ncm-tab', { active: store.ncmActiveTab === 'cookie' }]" @click="store.ncmActiveTab = 'cookie'">Cookie（推荐）</button>
          <button :class="['ncm-tab', { active: store.ncmActiveTab === 'phone' }]" @click="store.ncmActiveTab = 'phone'">手机号 + 密码</button>
        </div>
        <div :class="['ncm-panel', { active: store.ncmActiveTab === 'cookie' }]">
          <label>Cookie</label>
          <textarea v-model="store.ncmCookie" rows="4" placeholder="粘贴网易云 Cookie 字符串..."></textarea>
          <small>获取方式：浏览器打开 <b>music.163.com</b> 并登录 → F12 → Network → 任意请求 → Request Headers → Cookie</small>
        </div>
        <div :class="['ncm-panel', { active: store.ncmActiveTab === 'phone' }]">
          <label>手机号</label>
          <input type="text" v-model="store.ncmPhone" placeholder="186xxxxxxxx">
          <label>密码</label>
          <input type="password" v-model="store.ncmPassword" placeholder="网易云密码">
        </div>
        <div class="ncm-actions">
          <button class="btn btn-primary" @click="saveNcmSettings">保存</button>
          <button class="btn btn-info" @click="testNcmLogin">测试连接</button>
        </div>
        <div v-if="store.ncmResult" :style="{
          marginTop: '12px', padding: '10px 14px', borderRadius: '8px', fontSize: '0.88em', display: 'block',
          background: store.ncmResultType === 'success' ? 'rgba(40,167,69,0.15)' : store.ncmResultType === 'error' ? 'rgba(220,53,69,0.15)' : 'rgba(23,162,184,0.15)',
          color: store.ncmResultType === 'success' ? 'var(--success)' : store.ncmResultType === 'error' ? 'var(--danger)' : 'var(--info)'
        }" v-text="store.ncmResult"></div>
      </div>
    </div>

    <!-- Settings Panel -->
    <div v-show="subtab === 'settings'">
      <div class="card">
        <h2>⚙️ 系统设置</h2>
        <div class="form-group">
          <label>站点名称:</label>
          <input type="text" v-model="store.settingsStationName" placeholder="Rakuraku Music Station">
        </div>
        <div class="form-group">
          <label>副标题:</label>
          <input type="text" v-model="store.settingsSubtitle" placeholder="A Community Radio">
        </div>
        <div class="form-group color-group">
          <label>主题颜色:</label>
          <input type="color" v-model="store.settingsPrimaryColor"><span>主色</span>
          <input type="color" v-model="store.settingsSecondaryColor"><span>次色</span>
          <input type="color" v-model="store.settingsBgColor"><span>背景色</span>
        </div>
        <div class="form-group">
          <label>管理员密码:</label>
          <input type="password" v-model="store.settingsAdminPassword" placeholder="留空则不修改密码">
        </div>
        <div class="form-actions">
          <button class="btn btn-secondary" @click="loadSettings">加载设置</button>
          <button class="btn btn-primary" @click="saveSettings">保存设置</button>
        </div>
        <div v-if="store.settingsResult" :style="{
          marginTop: '15px', padding: '10px 14px', borderRadius: '6px', fontSize: '0.88em', display: 'block',
          background: store.settingsResultType === 'success' ? 'rgba(40,167,69,0.15)' : store.settingsResultType === 'error' ? 'rgba(220,53,69,0.15)' : 'rgba(23,162,184,0.15)',
          color: store.settingsResultType === 'success' ? 'var(--success)' : store.settingsResultType === 'error' ? 'var(--danger)' : 'var(--info)'
        }" v-text="store.settingsResult"></div>
      </div>
    </div>

    <!-- Stats Panel -->
    <div v-show="subtab === 'stats'">
      <div class="card admin-panel">
        <h2>📊 系统统计</h2>
        <div v-if="store.adminStats">
          👥 用户: {{ store.adminStats.users || 0 }} &nbsp;|&nbsp;
          🎵 歌曲: {{ store.adminStats.songs || 0 }} &nbsp;|&nbsp;
          📋 队列: {{ store.adminStats.queue_size || 0 }} &nbsp;|&nbsp;
          📁 歌单: {{ store.adminStats.playlists || 0 }}
        </div>
      </div>
    </div>
  </div>
</template>
