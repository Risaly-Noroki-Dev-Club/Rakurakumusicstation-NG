<script setup lang="ts">
import { store } from '../../store'
import { getBackendUrl } from '../../api'

async function loadSettings() {
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/settings')
    const data = await res.json()
    if (!data.success) { store.settingsResult = data.error || '加载失败'; store.settingsResultType = 'error'; return }
    const s = data.data
    store.settingsStationName = s.station_name || 'Rakuraku Music Station'
    store.settingsSubtitle = s.subtitle || ''
    store.settingsPrimaryColor = s.primary_color || '#003D99'
    store.settingsSecondaryColor = s.secondary_color || '#00897B'
    store.settingsBgColor = s.bg_color || '#FAFAFA'
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
      headers: { 'Content-Type': 'application/json' },
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

loadSettings()
</script>

<template>
  <div>
    <v-text-field
      v-model="store.settingsStationName"
      label="站点名称"
      placeholder="Rakuraku Music Station"
      class="mb-3"
    />
    <v-text-field
      v-model="store.settingsSubtitle"
      label="副标题"
      placeholder="A Community Radio"
      class="mb-3"
    />

    <div class="d-flex flex-wrap gap-4 align-center mb-3">
      <div class="d-flex align-center">
        <input
          v-model="store.settingsPrimaryColor"
          type="color"
          class="am-color-picker mr-2"
        >
        <span class="text-body-2">主色</span>
      </div>
      <div class="d-flex align-center">
        <input
          v-model="store.settingsSecondaryColor"
          type="color"
          class="am-color-picker mr-2"
        >
        <span class="text-body-2">次色</span>
      </div>
      <div class="d-flex align-center">
        <input
          v-model="store.settingsBgColor"
          type="color"
          class="am-color-picker mr-2"
        >
        <span class="text-body-2">背景色</span>
      </div>
    </div>

    <v-text-field
      v-model="store.settingsAdminPassword"
      label="管理员密码"
      placeholder="留空则不修改密码"
      type="password"
      class="mb-4"
    />

    <div class="d-flex gap-3">
      <v-btn variant="outlined" color="secondary" @click="loadSettings">
        加载设置
      </v-btn>
      <v-btn color="primary" @click="saveSettings">
        保存设置
      </v-btn>
    </div>

    <v-alert
      v-if="store.settingsResult"
      :type="store.settingsResultType as any"
      class="mt-4"
      density="compact"
      variant="tonal"
    >
      {{ store.settingsResult }}
    </v-alert>
  </div>
</template>

<style scoped>
.am-color-picker {
  width: 40px;
  height: 40px;
  border: 2px solid var(--am-border);
  border-radius: 6px;
  cursor: pointer;
  padding: 2px;
  background: var(--am-surface);
}

.gap-4 {
  gap: 16px;
}

.gap-3 {
  gap: 12px;
}
</style>
