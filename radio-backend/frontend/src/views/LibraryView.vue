<script setup lang="ts">
import { onMounted } from 'vue'
import { store, formatTime } from '../store'
import { debouncedSearch, addToQueue, loadMyPlaylists, createPlaylist } from '../api'

onMounted(() => { if (store.token) loadMyPlaylists() })
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
            <button v-if="store.currentUser" class="btn btn-secondary btn-small" @click="addToQueue(s.id)" title="收藏到歌单">⭐</button>
          </div>
        </div>
      </div>
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
  </div>
</template>
