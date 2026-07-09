<script setup lang="ts">
import { ref, computed } from 'vue'
import { store } from '../../store'
import { apiUrl, addToQueue, removeQueueItem, debouncedSearch } from '../../api'

const isAdmin = computed(() => store.deviceUser?.role === 'admin')

const showSearch = ref(false)
const searchQuery = ref('')

const searchResults = computed(() => store.searchResults)

function handleSearch() {
  store.searchQuery = searchQuery.value
  debouncedSearch()
}

function pickSong(songId: number) {
  addToQueue(songId)
  showSearch.value = false
  searchQuery.value = ''
  store.searchResults = []
}

function thumbSrc(songId: number, coverUrl?: string): string {
  if (coverUrl) return coverUrl
  return apiUrl('/api/songs/' + songId + '/cover')
}

function onRemove(id: number) {
  removeQueueItem(id)
}

function hideImg(e: Event) {
  const t = e.target as HTMLElement
  if (t) t.style.display = 'none'
}
</script>

<template>
  <div class="lt-queue-card">
    <div class="lt-queue-header">
      <span class="lt-queue-title">Play Next</span>
      <button class="lt-add-btn" @click="showSearch = true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
        <span>Add</span>
      </button>
    </div>

    <div v-if="store.queue.length === 0" class="lt-empty">队列为空</div>

    <div
      v-for="item in store.queue"
      :key="item.id"
      class="lt-queue-item"
      :class="{ playing: item.status === 'playing' }"
    >
      <div class="lt-queue-thumb">
        <img v-if="item.song_id" :src="thumbSrc(item.song_id, item.song?.cover_url)" @error="hideImg" />
        <svg v-else viewBox="0 0 24 24" fill="currentColor"><path d="M12 3v10.55A4 4 0 1 0 14 17V7h4V3h-6z" /></svg>
      </div>
      <div class="lt-queue-info">
        <div class="lt-queue-item-title">{{ item.song?.title || '未知歌曲' }}</div>
        <div class="lt-queue-item-artist">{{ item.song?.artist || '' }}</div>
      </div>
      <div class="lt-queue-actions">
        <span v-if="item.status === 'playing'" class="lt-playing-chip">播放中</span>
        <button
          v-if="isAdmin && item.status !== 'playing'"
          class="lt-queue-remove"
          title="移除"
          @click="onRemove(item.id)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>
    </div>

    <!-- 搜索点歌弹窗 -->
    <div v-if="showSearch" class="lt-search-overlay" @click.self="showSearch = false">
      <div class="lt-search-modal">
        <div class="lt-search-head">
          <input
            v-model="searchQuery"
            class="lt-search-input"
            placeholder="搜索歌曲并点歌..."
            autofocus
            @input="handleSearch"
          />
          <button class="lt-search-close" @click="showSearch = false">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
        <div class="lt-search-results">
          <div v-if="searchResults.length === 0 && searchQuery.trim() !== ''" class="lt-empty">
            无匹配结果
          </div>
          <div
            v-for="s in searchResults"
            :key="s.id"
            class="lt-search-item"
            @click="pickSong(s.id)"
          >
            <div class="lt-queue-thumb sm">
              <img v-if="s.cover_url" :src="s.cover_url" />
              <svg v-else viewBox="0 0 24 24" fill="currentColor"><path d="M12 3v10.55A4 4 0 1 0 14 17V7h4V3h-6z" /></svg>
            </div>
            <div class="lt-queue-info">
              <div class="lt-queue-item-title">{{ s.title }}</div>
              <div class="lt-queue-item-artist">{{ s.artist }}<span v-if="s.album"> · {{ s.album }}</span></div>
            </div>
            <svg class="lt-search-add" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="12" y1="5" x2="12" y2="19"></line>
              <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.lt-queue-card {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-md);
  padding: 16px;
  box-shadow: var(--lt-shadow-subtle);
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  position: relative;
}

.lt-queue-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.lt-queue-title {
  font-family: var(--lt-font-serif);
  font-size: 16px;
  font-weight: 700;
  color: var(--lt-text-primary);
}

.lt-add-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 12px;
  border-radius: 14px;
  border: 1px solid var(--lt-divider);
  background: transparent;
  font-size: 12px;
  font-weight: 500;
  color: var(--lt-text-secondary);
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.lt-add-btn:hover {
  background: var(--lt-btn-bg);
  border-color: var(--lt-selected-bg);
}

.lt-add-btn svg {
  width: 14px;
  height: 14px;
}

.lt-empty {
  font-size: 13px;
  color: var(--lt-text-muted);
  padding: 16px 4px;
  text-align: center;
}

.lt-queue-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 0;
}

.lt-queue-item:not(:last-child) {
  border-bottom: 1px solid var(--lt-divider);
}

.lt-queue-item.playing {
  background: var(--lt-accent-soft);
  border-radius: var(--lt-radius-sm);
  padding: 6px 8px;
}

.lt-queue-thumb {
  width: 48px;
  height: 48px;
  border-radius: var(--lt-radius-sm);
  background: var(--lt-btn-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
}

.lt-queue-thumb.sm {
  width: 40px;
  height: 40px;
}

.lt-queue-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.lt-queue-thumb svg {
  width: 18px;
  height: 18px;
  color: var(--lt-text-secondary);
}

.lt-queue-info {
  min-width: 0;
  flex: 1;
}

.lt-queue-item-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--lt-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-queue-item-artist {
  font-size: 12px;
  font-weight: 400;
  color: var(--lt-text-secondary);
  margin-top: 1px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-queue-actions {
  margin-left: auto;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 6px;
}

.lt-playing-chip {
  background: var(--lt-accent);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 10px;
  letter-spacing: 0.3px;
}

.lt-queue-remove {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: none;
  background: transparent;
  color: var(--lt-text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease;
  padding: 0;
}

.lt-queue-remove:hover {
  color: var(--lt-text-primary);
  background: var(--lt-btn-bg);
}

.lt-queue-remove svg {
  width: 14px;
  height: 14px;
}

/* 搜索弹窗 */
.lt-search-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 300;
  animation: lt-fade 0.2s ease;
}

.lt-search-modal {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-lg);
  width: 90%;
  max-width: 460px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--lt-shadow-card);
  overflow: hidden;
}

.lt-search-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--lt-divider);
}

.lt-search-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  font-size: 15px;
  color: var(--lt-text-primary);
  font-family: var(--lt-font-sans);
}

.lt-search-input::placeholder {
  color: var(--lt-text-muted);
}

.lt-search-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--lt-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  padding: 0;
}

.lt-search-close:hover {
  background: var(--lt-btn-bg);
}

.lt-search-close svg {
  width: 18px;
  height: 18px;
}

.lt-search-results {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.lt-search-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 4px;
  border-radius: var(--lt-radius-sm);
  cursor: pointer;
  transition: background 0.15s ease;
}

.lt-search-item:hover {
  background: var(--lt-btn-bg);
}

.lt-search-add {
  width: 16px;
  height: 16px;
  color: var(--lt-text-secondary);
  flex-shrink: 0;
}

@keyframes lt-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}
</style>
