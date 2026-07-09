<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { store } from '../store'
import { refreshQueue, removeQueueItem, addToQueue, debouncedSearch } from '../api'
import LtPageShell from '../components/lt/LtPageShell.vue'

onMounted(() => { refreshQueue() })

const searchQuery = ref('')

function handleSearch() {
  store.searchQuery = searchQuery.value
  debouncedSearch()
}

function replayQueueItem(songId: number) {
  addToQueue(songId)
}
</script>

<template>
  <!-- 桌面端：LT 风格 -->
  <LtPageShell v-if="store.isDesktop" title="Play Queue" subtitle="查看和管理待播歌曲">
    <!-- 搜索 -->
    <div class="lt-card lt-search-bar">
      <input v-model="searchQuery" placeholder="搜索歌曲并点歌..." class="lt-input" @input="handleSearch" />
    </div>

    <!-- 搜索结果 -->
    <div v-if="searchQuery.trim() !== '' && store.searchResults.length > 0" class="lt-card">
      <div class="lt-card-title">搜索结果 — 点击点歌</div>
      <div v-for="s in store.searchResults" :key="s.id" class="lt-song-item" @click="addToQueue(s.id)">
        <div class="lt-song-info">
          <div class="lt-song-title">{{ s.title }}</div>
          <div class="lt-song-artist">{{ (s.artist || '') + ' · ' + (s.album || '') }}</div>
        </div>
        <svg class="lt-add-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
      </div>
    </div>

    <!-- 待播队列 -->
    <div class="lt-card">
      <div class="lt-card-title-row">
        <span class="lt-card-title">待播队列</span>
        <span class="lt-badge">{{ store.queue.length }} 首</span>
      </div>
      <div v-if="store.queue.length === 0" class="lt-empty">队列为空</div>
      <div v-for="item in store.queue" :key="item.id" class="lt-song-item" :class="{ playing: item.status === 'playing' }">
        <div class="lt-song-info">
          <div class="lt-song-title">{{ item.song?.title || '未知歌曲' }}</div>
          <div class="lt-song-artist">{{ store.deviceUser?.role === 'admin' ? (item.song?.artist || '') + ' · 点歌: ' + item.requested_by : (item.song?.artist || '') }}</div>
        </div>
        <div class="lt-song-actions">
          <span v-if="item.status === 'playing'" class="lt-chip active">播放中</span>
          <span v-else class="lt-chip">等待</span>
          <button v-if="item.status !== 'playing' && item.song?.id" class="lt-icon-btn" title="再播放一次" @click="replayQueueItem(item.song.id)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg>
          </button>
          <button v-if="store.deviceUser?.role === 'admin' && item.status !== 'playing'" class="lt-icon-btn danger" @click="removeQueueItem(item.id)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
      </div>
    </div>

    <!-- 播放历史 -->
    <div v-if="store.deviceUser?.role === 'admin'" class="lt-card">
      <div class="lt-card-title-row">
        <span class="lt-card-title">播放历史</span>
        <span class="lt-badge">{{ store.history.length }} 首</span>
      </div>
      <div v-if="store.history.length === 0" class="lt-empty-sm">暂无历史</div>
      <div v-for="h in store.history" :key="h.id" class="lt-song-item">
        <div class="lt-song-info">
          <div class="lt-song-title">{{ h.song?.title || '未知' }}</div>
          <div class="lt-song-artist">{{ (h.requested_by || '') + ' · ' + (h.played_at || '') }}</div>
        </div>
        <button v-if="h.song?.id" class="lt-icon-btn" title="再播放一次" @click="replayQueueItem(h.song.id)">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg>
        </button>
      </div>
    </div>
  </LtPageShell>

  <!-- 移动端：原有布局 -->
  <div v-else class="am-up-next">
    <!-- Page Header -->
    <div class="am-page-header mb-5">
      <h1 class="text-h5 font-weight-bold">队列</h1>
      <p class="text-body-2 text-medium-emphasis mt-1">查看和管理待播歌曲</p>
    </div>

    <!-- Search -->
    <v-card class="mb-5 am-card" elevation="0">
      <v-card-text class="pa-4">
        <v-text-field
          v-model="searchQuery"
          placeholder="搜索歌曲并点歌..."
          prepend-inner-icon="mdi-magnify"
          clearable
          hide-details
          density="comfortable"
          @update:model-value="handleSearch"
        />
      </v-card-text>
    </v-card>

    <!-- Quick add from search results -->
    <v-card v-if="searchQuery.trim() !== '' && store.searchResults.length > 0" class="mb-5 am-card" elevation="0">
      <v-card-title class="text-subtitle-1 font-weight-bold pa-4 pb-2">
        搜索结果 — 点击点歌
      </v-card-title>
      <v-card-text class="pa-0">
        <v-list lines="two">
          <v-list-item
            v-for="s in store.searchResults"
            :key="s.id"
            :title="s.title"
            :subtitle="(s.artist || '') + ' · ' + (s.album || '')"
            @click="addToQueue(s.id)"
            class="am-queue-item cursor-pointer"
          >
            <template #prepend>
              <v-icon color="primary" size="20">mdi-plus-circle-outline</v-icon>
            </template>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>

    <!-- Queue -->
    <v-card class="mb-5 am-card" elevation="0">
      <v-card-title class="text-subtitle-1 font-weight-bold d-flex align-center pa-4 pb-2">
        待播队列
        <v-spacer />
        <v-chip size="small" color="primary" variant="tonal" rounded="lg">
          {{ store.queue.length }} 首
        </v-chip>
      </v-card-title>
      <v-card-text class="pa-0">
        <div v-if="store.queue.length === 0" class="text-center py-12 text-medium-emphasis">
          <v-icon size="48" color="disabled" class="mb-3">mdi-format-list-numbered</v-icon>
          <div class="text-body-2">队列为空</div>
        </div>
        <v-list v-else lines="two">
          <v-list-item
            v-for="item in store.queue"
            :key="item.id"
            :title="item.song?.title || '未知歌曲'"
            :subtitle="store.deviceUser?.role === 'admin' ? (item.song?.artist || '') + ' · 点歌: ' + item.requested_by : (item.song?.artist || '')"
            :class="{ 'am-playing-item': item.status === 'playing' }"
          >
            <template #prepend>
              <div v-if="item.status === 'playing'" class="am-playing-indicator">
                <v-icon color="primary" size="20">mdi-volume-high</v-icon>
              </div>
              <v-icon v-else color="disabled" size="18">mdi-music-note-outline</v-icon>
            </template>
            <template #append>
              <v-chip
                v-if="item.status === 'playing'"
                size="x-small"
                color="success"
                variant="flat"
                rounded="lg"
              >
                播放中
              </v-chip>
              <v-chip
                v-else
                size="x-small"
                color="medium-emphasis"
                variant="tonal"
                rounded="lg"
              >
                等待
              </v-chip>
              <v-btn
                v-if="item.status !== 'playing' && item.song?.id"
                icon
                variant="text"
                color="primary"
                size="x-small"
                class="ml-2"
                title="再播放一次"
                @click="replayQueueItem(item.song.id)"
              >
                <v-icon size="16">mdi-replay</v-icon>
              </v-btn>
              <v-btn
                v-if="store.deviceUser?.role === 'admin' && item.status !== 'playing'"
                icon
                variant="text"
                color="error"
                size="x-small"
                class="ml-1"
                @click="removeQueueItem(item.id)"
              >
                <v-icon size="16">mdi-close</v-icon>
              </v-btn>
            </template>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>

    <!-- History -->
    <v-card v-if="store.deviceUser?.role === 'admin'" class="am-card" elevation="0">
      <v-card-title class="text-subtitle-1 font-weight-bold d-flex align-center pa-4 pb-2">
        播放历史
        <v-spacer />
        <v-chip size="small" color="secondary" variant="tonal" rounded="lg">
          {{ store.history.length }} 首
        </v-chip>
      </v-card-title>
      <v-card-text class="pa-0">
        <div v-if="store.history.length === 0" class="text-center py-8 text-medium-emphasis">
          <v-icon size="40" color="disabled" class="mb-2">mdi-history</v-icon>
          <div class="text-caption">暂无历史</div>
        </div>
        <v-list v-else lines="two" class="max-h-300 overflow-y-auto">
          <v-list-item
            v-for="h in store.history"
            :key="h.id"
            :title="h.song?.title || '未知'"
            :subtitle="(h.requested_by || '') + ' · ' + (h.played_at || '')"
          >
            <template #prepend>
              <v-icon color="disabled" size="18">mdi-history</v-icon>
            </template>
            <template #append>
              <v-btn
                v-if="h.song?.id"
                icon
                variant="tonal"
                color="primary"
                size="x-small"
                title="再播放一次"
                @click="replayQueueItem(h.song.id)"
              >
                <v-icon size="16">mdi-replay</v-icon>
              </v-btn>
            </template>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>
/* ─── LT 桌面端样式 ─── */
.lt-card {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-md);
  padding: 16px;
  box-shadow: var(--lt-shadow-subtle);
}

.lt-search-bar { flex-shrink: 0; }

.lt-input {
  width: 100%;
  border: none;
  outline: none;
  background: transparent;
  font-size: 15px;
  color: var(--lt-text-primary);
  font-family: var(--lt-font-sans);
  padding: 4px 0;
}
.lt-input::placeholder { color: var(--lt-text-muted); }

.lt-card-title {
  font-family: var(--lt-font-serif);
  font-size: 16px;
  font-weight: 700;
  color: var(--lt-text-primary);
  margin-bottom: 12px;
}

.lt-card-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.lt-badge {
  font-size: 12px;
  font-weight: 600;
  color: var(--lt-text-secondary);
  background: var(--lt-btn-bg);
  padding: 2px 8px;
  border-radius: 10px;
}

.lt-empty {
  font-size: 13px;
  color: var(--lt-text-muted);
  padding: 32px 4px;
  text-align: center;
}

.lt-empty-sm {
  font-size: 12px;
  color: var(--lt-text-muted);
  padding: 16px 0;
  text-align: center;
}

.lt-song-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 4px;
  border-bottom: 1px solid var(--lt-divider);
  cursor: default;
}
.lt-song-item:last-child { border-bottom: none; }
.lt-song-item.playing { background: var(--lt-accent-soft); border-radius: var(--lt-radius-sm); padding: 8px 8px; }
.lt-song-item[onclick], .lt-song-item[role] { cursor: pointer; }

.lt-song-info { flex: 1; min-width: 0; }

.lt-song-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--lt-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-song-artist {
  font-size: 12px;
  color: var(--lt-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 1px;
}

.lt-song-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.lt-chip {
  font-size: 10px;
  font-weight: 600;
  color: var(--lt-text-secondary);
  background: var(--lt-btn-bg);
  padding: 2px 8px;
  border-radius: 10px;
}

.lt-chip.active {
  background: var(--lt-accent);
  color: #fff;
}

.lt-icon-btn {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  border: none;
  background: transparent;
  color: var(--lt-text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  padding: 0;
}
.lt-icon-btn:hover { background: var(--lt-btn-bg); color: var(--lt-text-primary); }
.lt-icon-btn.danger:hover { color: #e53935; }
.lt-icon-btn svg { width: 16px; height: 16px; }

.lt-add-icon {
  width: 16px;
  height: 16px;
  color: var(--lt-text-secondary);
  flex-shrink: 0;
}

/* ─── 移动端原有样式 ─── */
.am-up-next {
  padding-bottom: 16px;
  animation: slideUp 0.5s var(--am-ease-emphasized);
}

.am-page-header {
  animation: slideUp 0.4s var(--am-ease-emphasized);
}

.am-card {
  overflow: hidden;
}

.am-card:hover {
  border-color: transparent;
}

.am-playing-item {
  background: rgba(108, 92, 231, 0.06) !important;
  border-left: 3px solid var(--am-primary);
}

.am-playing-indicator {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(108, 92, 231, 0.1);
  border-radius: var(--am-radius-sm);
  margin-right: 8px;
}

.am-queue-item {
  cursor: pointer;
  transition: background-color 0.2s var(--am-ease-emphasized);
}

.am-queue-item:hover {
  background: var(--am-surface-2);
}

.max-h-300 {
  max-height: 300px;
}

.cursor-pointer {
  cursor: pointer;
}

@keyframes slideUp {
  from { transform: translateY(16px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
