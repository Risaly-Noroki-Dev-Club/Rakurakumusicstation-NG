<script setup lang="ts">
import { store } from '../../store'
import { getBackendUrl } from '../../api'
import StatusMessage from '../StatusMessage.vue'

function authHeaders() {
  const h: Record<string, string> = {}; if (store.token) h.Authorization = "Bearer " + store.token; return h
}

async function loadSettings() {
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/settings', { headers: authHeaders() })
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

const emit = defineEmits<{ load: [] }>()
loadSettings()
</script>

<template>
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
    <StatusMessage :message="store.settingsResult" :type="store.settingsResultType as any" />
  </div>
</template>
