<script setup lang="ts">
import { ref } from 'vue'
import { store } from '../../store'
import { apiFetch } from '../../api'

const iconInput = ref<HTMLInputElement | null>(null)

async function loadSettings() {
  try {
    const res = await apiFetch('/api/admin/settings')
    const data = await res.json()
    if (!data.success) { store.settingsResult = data.error || '加载失败'; store.settingsResultType = 'error'; return }
    const s = data.data
    store.settingsStationName = s.station_name || 'Rakuraku Music Station'
    store.settingsShortName = s.short_name || 'RakurakuRadio'
    store.settingsSubtitle = s.subtitle || ''
    store.settingsDescription = s.description || ''
    store.settingsIconUrl = s.icon_url || ''
    store.settingsResolvedIconUrl = s.resolved_icon_url || ''
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
      short_name: store.settingsShortName.trim(),
      subtitle: store.settingsSubtitle.trim(),
      description: store.settingsDescription.trim(),
      icon_url: store.settingsIconUrl.trim(),
    }
    if (store.settingsAdminPassword.trim()) {
      settings.admin_password = store.settingsAdminPassword.trim()
    }
    const res = await apiFetch('/api/admin/settings', {
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

async function uploadIcon(file: File) {
  const form = new FormData()
  form.append('file', file)
  store.settingsResult = '正在上传图标...'
  store.settingsResultType = 'info'
  try {
    const res = await apiFetch('/api/admin/settings/icon/', {
      method: 'POST',
      body: form,
    })
    const data = await res.json()
    if (data.success) {
      store.settingsResult = data.data || '图标已上传'
      store.settingsResultType = 'success'
      store.settingsIconFileName = file.name
      await loadSettings()
    } else {
      store.settingsResult = '上传失败: ' + (data.error || '')
      store.settingsResultType = 'error'
    }
  } catch (e: any) {
    store.settingsResult = '上传失败: ' + (e && e.message)
    store.settingsResultType = 'error'
  }
}

function onIconSelected(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) uploadIcon(file)
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
      v-model="store.settingsShortName"
      label="短名称"
      placeholder="RakurakuRadio"
      hint="用于 PWA 安装名称等较短展示场景"
      class="mb-3"
    />
    <v-text-field
      v-model="store.settingsSubtitle"
      label="副标题"
      placeholder="A Community Radio"
      class="mb-3"
    />
    <v-textarea
      v-model="store.settingsDescription"
      label="描述"
      placeholder="Community Radio - Low Latency Audio Streaming"
      rows="2"
      class="mb-3"
    />
    <v-text-field
      v-model="store.settingsIconUrl"
      label="图标 URL"
      placeholder="https://example.com/icon.png"
      hint="留空则使用上传图标或默认图标"
      class="mb-3"
    />
    <div class="d-flex flex-wrap gap-3 align-center mb-4">
      <v-avatar v-if="store.settingsResolvedIconUrl" size="48" rounded="lg" class="am-icon-preview">
        <v-img :src="store.settingsResolvedIconUrl" cover />
      </v-avatar>
      <input
        ref="iconInput"
        type="file"
        accept="image/png,image/svg+xml,image/webp,image/jpeg"
        style="display: none"
        @change="onIconSelected"
      >
      <v-btn variant="outlined" prepend-icon="mdi-image" @click="iconInput?.click()">
        上传图标
      </v-btn>
      <span class="text-caption text-medium-emphasis">PNG / SVG / WebP / JPEG，最大 2MB</span>
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
.gap-3 {
  gap: 12px;
}

.am-icon-preview {
  border: 1px solid var(--am-divider);
}
</style>
