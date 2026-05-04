<script setup lang="ts">
import { ref } from 'vue'
import { store, toast } from '../../store'
import { getBackendUrl } from '../../api'
import StatusMessage from '../StatusMessage.vue'

const uploadFile = ref<File | null>(null)
const uploadFileName = ref('')
const uploadStatus = ref('')
const uploadStatusType = ref('')

function handleFileSelect(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) {
    uploadFile.value = file
    uploadFileName.value = file.name
  }
}

async function uploadSong() {
  if (!uploadFile.value) return
  if (uploadFile.value.size > 100 * 1024 * 1024) {
    uploadStatus.value = '文件大小超过 100MB 限制'
    uploadStatusType.value = 'error'
    return
  }
  const formData = new FormData()
  formData.append('file', uploadFile.value)
  uploadStatus.value = '上传中...'
  uploadStatusType.value = 'info'
  try {
    const res = await fetch(getBackendUrl() + '/api/admin/upload', {
      method: 'POST',
      headers: { 'Authorization': 'Bearer ' + (store.token || '') },
      body: formData
    })
    const data = await res.json()
    if (data.success) {
      uploadStatus.value = '✅ ' + data.data
      uploadStatusType.value = 'success'
      uploadFile.value = null
      uploadFileName.value = ''
    } else {
      uploadStatus.value = '❌ ' + (data.error || '上传失败')
      uploadStatusType.value = 'error'
    }
  } catch {
    uploadStatus.value = '❌ 上传失败'
    uploadStatusType.value = 'error'
  }
}
</script>

<template>
  <div class="card">
    <h2>📤 上传新音乐</h2>
    <div class="upload-section">
      <input type="file" class="upload-file-input" accept=".mp3,.wav,.flac,.ogg,.m4a,.aac" @change="handleFileSelect">
      <button class="btn btn-primary" @click="uploadSong" :disabled="!uploadFile">上传文件</button>
    </div>
    <StatusMessage :message="uploadStatus" :type="uploadStatusType as any" />
  </div>
</template>
