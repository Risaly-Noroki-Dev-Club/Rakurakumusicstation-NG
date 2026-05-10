<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { store, toast } from '../../store'
import { getBackendUrl } from '../../api'

let eventSource: EventSource | null = null
const fileInput = ref<HTMLInputElement | null>(null)

function startDownload() {
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

  fetch(getBackendUrl() + '/api/admin/download', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ playlist, quality: store.downloadQuality, format: store.downloadFormat })
  })
    .then(res => res.json())
    .then(data => {
      if (data.success) {
        store.downloadStatusMsg = data.data || '任务已提交'
        store.downloadStatusType = 'info'
        openEventSource()
      } else {
        store.downloadStatusMsg = data.error || '启动失败'
        store.downloadStatusType = 'error'
        store.downloadRunning = false
      }
    })
    .catch(() => {
      store.downloadStatusMsg = '请求失败'
      store.downloadStatusType = 'error'
      store.downloadRunning = false
    })
}

function openEventSource() {
  if (eventSource) eventSource.close()
  const url = getBackendUrl() + '/api/admin/download/stream'
  eventSource = new EventSource(url, { withCredentials: true })
  eventSource.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data)
      if (data.log) store.downloadLog += data.log + '\n'
      if (data.done) {
        store.downloadRunning = false
        store.downloadStatusMsg = '下载完成'
        store.downloadStatusType = 'success'
        closeEventSource()
        toast('下载完成，播放队列已刷新', 'success')
      }
    } catch {
      store.downloadLog += e.data + '\n'
    }
  }
  eventSource.onerror = () => {
    if (!store.downloadRunning) closeEventSource()
  }
}

function closeEventSource() {
  if (eventSource) {
    eventSource.close()
    eventSource = null
  }
}

onUnmounted(() => {
  closeEventSource()
})
</script>

<template>
  <div>
    <v-textarea
      v-model="store.downloadPlaylist"
      rows="8"
      placeholder="每行一首，格式：艺术家 - 歌名&#10;例：&#10;toe - tremolo+delay&#10;Whale Fall - True Places"
      hide-details
      class="mb-3"
    />

    <div class="d-flex flex-wrap gap-3 align-center mb-3">
      <v-select
        v-model="store.downloadQuality"
        :items="[
          { title: '超高音质 (320k)', value: 'exhigh' },
          { title: '无损', value: 'lossless' },
          { title: '高音质 (192k)', value: 'high' },
          { title: '标准 (128k)', value: 'standard' },
        ]"
        label="音质"
        item-title="title"
        item-value="value"
        style="min-width: 180px"
        hide-details
      />
      <v-select
        v-model="store.downloadFormat"
        :items="[
          { title: 'MP3', value: 'mp3' },
          { title: 'FLAC', value: 'flac' },
          { title: 'M4A', value: 'm4a' },
          { title: 'Opus', value: 'opus' },
        ]"
        label="格式"
        item-title="title"
        item-value="value"
        style="min-width: 120px"
        hide-details
      />
      <v-spacer />
      <v-btn
        color="primary"
        prepend-icon="mdi-download"
        :disabled="store.downloadRunning"
        @click="startDownload"
      >
        开始下载
      </v-btn>
    </div>

    <v-alert
      v-if="store.downloadStatusMsg"
      :type="store.downloadStatusType as any"
      density="compact"
      variant="tonal"
      class="mb-3"
    >
      {{ store.downloadStatusMsg }}
    </v-alert>

    <v-card
      v-if="store.downloadLog"
      class="mt-3"
      color="surface-variant"
      elevation="0"
    >
      <v-card-text>
        <pre class="text-caption" style="max-height: 280px; overflow-y: auto; white-space: pre-wrap; word-break: break-all; font-family: var(--font-mono);">{{ store.downloadLog }}</pre>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>
.gap-3 {
  gap: 12px;
}
</style>
