<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { store, toast } from '../../store'
import { getBackendUrl } from '../../api'
import StatusMessage from '../StatusMessage.vue'

let eventSource: EventSource | null = null

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
        store.downloadStatusMsg = '❌ ' + (data.error || '启动失败')
        store.downloadStatusType = 'error'
        store.downloadRunning = false
      }
    })
    .catch(() => {
      store.downloadStatusMsg = '❌ 请求失败'
      store.downloadStatusType = 'error'
      store.downloadRunning = false
    })
}

function openEventSource() {
  if (eventSource) {
    eventSource.close()
  }

  const url = getBackendUrl() + '/api/admin/download/stream'
  eventSource = new EventSource(url, { withCredentials: true })

  eventSource.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data)
      if (data.log) {
        store.downloadLog += data.log + '\n'
      }
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
    if (!store.downloadRunning) {
      closeEventSource()
    }
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
    <StatusMessage :message="store.downloadStatusMsg" :type="store.downloadStatusType as any" />
    <div v-if="store.downloadLog" class="download-log" style="display:block" v-text="store.downloadLog"
         :style="{ background: 'var(--card)', color: 'var(--text)', padding: '14px', borderRadius: '8px', fontSize: '0.82em', maxHeight: '280px', overflowY: 'auto', whiteSpace: 'pre-wrap', wordBreak: 'break-all', fontFamily: 'monospace', marginTop: '12px' }"></div>
  </div>
</template>
