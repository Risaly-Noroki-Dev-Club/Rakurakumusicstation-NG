<script setup lang="ts">
import { onMounted } from 'vue'
import { store, formatTime } from '../store'
import { debouncedSearch, addToQueue, downloadSong, uploadSong, loadMyPlaylists, createPlaylist,
         loadUserNcmStatus, saveUserNcmSettings, testUserNcmLogin, onSearchInput } from '../api'

onMounted(() => {
  if (store.token) {
    loadMyPlaylists()
    loadUserNcmStatus()
  }
})

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

async function handleUpload() {
  if (!store.uploadFile) {
    store.uploadStatus = '请选择文件'
    store.uploadStatusType = 'error'
    return
  }
  store.uploadStatus = '上传中...'
  store.uploadStatusType = 'info'
  const ok = await uploadSong(store.uploadFile)
  if (ok) {
    store.uploadStatus = '上传成功'
    store.uploadStatusType = 'success'
    store.uploadFile = null
    store.uploadFileName = ''
    const fileInput = document.querySelector('.upload-file-input') as HTMLInputElement | null
    if (fileInput) fileInput.value = ''
    onSearchInput()
  } else {
    store.uploadStatus = '上传失败'
    store.uploadStatusType = 'error'
  }
}
</script>

<template>
  <div>
    <div class="card">
      <h2>🔍 曲库搜索</h2>
      <input type="text" class="search-box" v-model="store.searchQuery"
             placeholder="搜索歌曲或艺术家..." @input="debouncedSearch">
      <div v-if="store.searchQuery.trim() === ''" style="text-align:center;color:var(--text-muted);padding:20px">
        输入关键词搜索曲库
      </div>
      <div v-else-if="store.searchResults.length === 0" style="text-align:center;color:var(--text-muted);padding:20px">
        未找到匹配的歌曲
      </div>
      <div v-else class="song-list">
        <div v-for="s in store.searchResults" :key="s.id" class="song-item">
          <div>
            <div style="font-weight:600">{{ s.title }}</div>
            <div style="font-size:0.85em;color:var(--text-muted)">
              {{ s.artist }} · {{ s.album }} · {{ formatTime(s.duration_ms) }}
            </div>
          </div>
          <div style="display:flex;gap:4px">
            <button class="btn btn-primary btn-small" @click="addToQueue(s.id)" title="投喂到电台">📻 点歌</button>
            <button v-if="store.currentUser" class="btn btn-secondary btn-small" @click="downloadSong(s.id)" title="下载">⬇️</button>
          </div>
        </div>
      </div>
    </div>

    <div v-if="store.currentUser" class="card">
      <h2>📤 上传歌曲</h2>
      <div class="upload-section">
        <input type="file" class="upload-file-input" accept=".mp3,.wav,.flac,.ogg,.m4a,.aac" @change="handleFileSelect">
        <button class="btn btn-primary" @click="handleUpload" :disabled="!store.uploadFile">上传文件</button>
      </div>
      <div v-if="store.uploadStatus" :style="{
        marginTop: '12px', padding: '10px 14px', borderRadius: '8px', fontSize: '0.88em', display: 'block',
        background: store.uploadStatusType === 'success' ? 'rgba(40,167,69,0.15)' : store.uploadStatusType === 'error' ? 'rgba(220,53,69,0.15)' : 'rgba(23,162,184,0.15)',
        color: store.uploadStatusType === 'success' ? 'var(--success)' : store.uploadStatusType === 'error' ? 'var(--danger)' : 'var(--info)'
      }" v-text="store.uploadStatus"></div>
    </div>

    <div v-if="store.currentUser" class="card">
      <h2>📁 我的歌单</h2>
      <div v-if="store.myPlaylists.length === 0" style="text-align:center;color:var(--text-muted);padding:10px">还没有歌单</div>
      <div v-for="pl in store.myPlaylists" :key="pl.id" class="queue-item" style="cursor:default">
        <div class="info">
          <span>{{ pl.name }}</span>
          <span style="color:var(--text-muted);font-size:0.85em"> ({{ pl.song_count }}首)</span>
        </div>
      </div>
      <div style="margin-top:10px;display:flex;gap:8px">
        <input type="text" v-model="store.newPlaylistName" placeholder="新建歌单名称"
               style="flex-grow:1;padding:8px;border:2px solid var(--border);border-radius:8px;background:var(--card);color:var(--text)">
        <button class="btn btn-primary btn-small" @click="createPlaylist">创建</button>
      </div>
    </div>

    <div v-if="store.currentUser" class="card">
      <h2>🎵 我的网易云账号</h2>
      <p style="color:var(--text-muted);font-size:0.85em;margin-bottom:12px">登录后可下载 VIP 歌曲。Cookie 方式更稳定，手机号方式可能触发验证码。</p>
      <div :class="['ncm-badge', store.userNcmBadgeClass]" v-text="store.userNcmBadge"></div>
      <div class="ncm-tabs">
        <button :class="['ncm-tab', { active: store.userNcmActiveTab === 'cookie' }]" @click="store.userNcmActiveTab = 'cookie'">Cookie（推荐）</button>
        <button :class="['ncm-tab', { active: store.userNcmActiveTab === 'phone' }]" @click="store.userNcmActiveTab = 'phone'">手机号 + 密码</button>
      </div>
      <div :class="['ncm-panel', { active: store.userNcmActiveTab === 'cookie' }]">
        <label>Cookie</label>
        <textarea v-model="store.userNcmCookie" rows="4" placeholder="粘贴网易云 Cookie 字符串..."></textarea>
        <small>获取方式：浏览器打开 <b>music.163.com</b> 并登录 → F12 → Network → 任意请求 → Request Headers → Cookie</small>
      </div>
      <div :class="['ncm-panel', { active: store.userNcmActiveTab === 'phone' }]">
        <label>手机号</label>
        <input type="text" v-model="store.userNcmPhone" placeholder="186xxxxxxxx">
        <label>密码</label>
        <input type="password" v-model="store.userNcmPassword" placeholder="网易云密码">
      </div>
      <div class="ncm-actions">
        <button class="btn btn-primary" @click="saveUserNcmSettings">保存</button>
        <button class="btn btn-info" @click="testUserNcmLogin">测试连接</button>
      </div>
      <div v-if="store.userNcmResult" :style="{
        marginTop: '12px', padding: '10px 14px', borderRadius: '8px', fontSize: '0.88em', display: 'block',
        background: store.userNcmResultType === 'success' ? 'rgba(40,167,69,0.15)' : store.userNcmResultType === 'error' ? 'rgba(220,53,69,0.15)' : 'rgba(23,162,184,0.15)',
        color: store.userNcmResultType === 'success' ? 'var(--success)' : store.userNcmResultType === 'error' ? 'var(--danger)' : 'var(--info)'
      }" v-text="store.userNcmResult"></div>
    </div>
  </div>
</template>
