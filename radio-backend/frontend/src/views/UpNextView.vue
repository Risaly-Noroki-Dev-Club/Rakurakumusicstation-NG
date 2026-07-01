<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { store } from '../store'
import { refreshQueue, removeQueueItem, addToQueue, debouncedSearch } from '../api'

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
  <div class="am-up-next">
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
