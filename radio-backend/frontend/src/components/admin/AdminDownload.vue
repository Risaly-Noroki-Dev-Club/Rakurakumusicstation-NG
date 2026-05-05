<script setup lang="ts">
import { onUnmounted } from 'vue'
import { store, toast } from '../../store'
import { getBackendUrl } from '../../api'
import StatusMessage from '../StatusMessage.vue'

let downloadPoller: ReturnType<typeof setInterval> | null = null

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
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ playlist, quality: store.downloadQuality, format: store.downloadFormat })
    })
    const data = await res.json()
    if (data.success) {
      store.downloadStatusMsg = data.data || '任务已提交，正在下载中...'
      store.downloadStatusType = 'info'
      pollDownload()
    } else {
      store.downloadStatusMsg = '❌ ' + (data.error || '启动失败')
      store.downloadStatusType = 'error'
      store.downloadRunning = false
    }
  } catch {
    store.downloadStatusMsg = '❌ 请求失败'
    store.downloadStatusType = 'error'
    store.downloadRunning = false
  }
}

function pollDownload() {
  if (downloadPoller) clearInterval(downloadPoller)
  downloadPoller = setInterval(async () => {
    try {
      const res = await fetch(getBackendUrl() + '/api/admin/download/status')
      const data = await res.json()
      if (data.success) {
        const d = data.data
        store.downloadStatusMsg = d.status || '运行中...'
        store.downloadLog = d.log || ''
        if (d.status && (d.status.includes('完成') || d.status.includes('错误'))) {
          store.downloadRunning = false
          store.downloadStatusType = d.status.includes('完成') ? 'success' : 'error'
          if (downloadPoller) { clearInterval(downloadPoller); downloadPoller = null }
        }
      }
    } catch { /* ignore */ }
  }, 2000)
}

onUnmounted(() => { if (downloadPoller) clearInterval(downloadPoller) })
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
