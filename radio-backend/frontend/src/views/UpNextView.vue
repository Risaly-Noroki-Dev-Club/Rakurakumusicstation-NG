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
    <!-- Search -->
    <v-card class="mb-4" elevation="1">
      <v-card-text>
        <v-text-field
          v-model="searchQuery"
          placeholder="搜索歌曲并点歌..."
          prepend-inner-icon="mdi-magnify"
          clearable
          hide-details
          @update:model-value="handleSearch"
        />
      </v-card-text>
    </v-card>

    <!-- Quick add from search results -->
    <v-card v-if="searchQuery.trim() !== '' && store.searchResults.length > 0" class="mb-4" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold">
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
            class="cursor-pointer"
          >
            <template #append>
              <v-icon color="primary">mdi-plus-circle</v-icon>
            </template>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>

    <!-- Queue -->
    <v-card class="mb-4" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold d-flex align-center">
        待播队列
        <v-spacer />
        <v-chip size="small" color="primary" variant="tonal">
          {{ store.queue.length }} 首
        </v-chip>
      </v-card-title>
      <v-card-text class="pa-0">
        <div v-if="store.queue.length === 0" class="text-center py-8 text-medium-emphasis">
          队列为空
        </div>
        <v-list v-else lines="two">
          <v-list-item
            v-for="item in store.queue"
            :key="item.id"
            :title="item.song?.title || '未知歌曲'"
            :subtitle="store.deviceUser?.role === 'admin' ? (item.song?.artist || '') + ' · 点歌: ' + item.requested_by : (item.song?.artist || '')"
            :class="{ 'bg-primary-lighten-5': item.status === 'playing' }"
          >
            <template #prepend>
              <v-icon
                v-if="item.status === 'playing'"
                color="primary"
                size="small"
              >
                mdi-volume-high
              </v-icon>
              <v-icon v-else color="disabled" size="small">mdi-music-note</v-icon>
            </template>
            <template #append>
              <v-chip
                v-if="item.status === 'playing'"
                size="x-small"
                color="success"
                variant="flat"
              >
                播放中
              </v-chip>
              <v-chip
                v-else
                size="x-small"
                color="medium-emphasis"
                variant="tonal"
              >
                等待
              </v-chip>
              <v-btn
                v-if="item.status !== 'playing' && item.song?.id"
                icon
                variant="text"
                color="primary"
                size="x-small"
                class="ml-1"
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
    <v-card v-if="store.deviceUser?.role === 'admin'" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold d-flex align-center">
        播放历史
        <v-spacer />
        <v-chip size="small" color="secondary" variant="tonal">
          {{ store.history.length }} 首
        </v-chip>
      </v-card-title>
      <v-card-text class="pa-0">
        <div v-if="store.history.length === 0" class="text-center py-8 text-medium-emphasis">
          暂无历史
        </div>
        <v-list v-else lines="two" class="max-h-300 overflow-y-auto">
          <v-list-item
            v-for="h in store.history"
            :key="h.id"
            :title="h.song?.title || '未知'"
            :subtitle="(h.requested_by || '') + ' · ' + (h.played_at || '')"
          >
            <template #prepend>
              <v-icon color="disabled" size="small">mdi-history</v-icon>
            </template>
            <template #append>
              <v-btn
                v-if="h.song?.id"
                icon
                variant="text"
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
}

.max-h-300 {
  max-height: 300px;
}

.bg-primary-lighten-5 {
  background-color: rgba(0, 61, 153, 0.05);
}

.cursor-pointer {
  cursor: pointer;
}
</style>
