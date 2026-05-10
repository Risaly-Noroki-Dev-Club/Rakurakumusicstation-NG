<script setup lang="ts">
import { ref } from 'vue'
import { toast } from '../../store'
import { getBackendUrl } from '../../api'

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
      body: formData
    })
    const data = await res.json()
    if (data.success) {
      uploadStatus.value = data.data || '上传成功'
      uploadStatusType.value = 'success'
      uploadFile.value = null
      uploadFileName.value = ''
    } else {
      uploadStatus.value = data.error || '上传失败'
      uploadStatusType.value = 'error'
    }
  } catch {
    uploadStatus.value = '上传失败'
    uploadStatusType.value = 'error'
  }
}
</script>

<template>
  <div>
    <input
      type="file"
      accept=".mp3,.wav,.flac,.ogg,.m4a,.aac"
      style="display: none"
      @change="handleFileSelect"
      ref="fileInput"
    >
    <v-btn
      variant="outlined"
      color="primary"
      prepend-icon="mdi-cloud-upload"
      block
      @click="($refs.fileInput as HTMLInputElement).click()"
    >
      {{ uploadFileName || '选择文件' }}
    </v-btn>

    <v-btn
      v-if="uploadFile"
      class="mt-3"
      color="primary"
      block
      @click="uploadSong"
    >
      上传
    </v-btn>

    <v-alert
      v-if="uploadStatus"
      :type="uploadStatusType as any"
      class="mt-3"
      density="compact"
      variant="tonal"
    >
      {{ uploadStatus }}
    </v-alert>
  </div>
</template>
