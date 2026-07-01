<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { store, formatTime } from '../store'
import {
  debouncedSearch, addToQueue, downloadSong,
  loadMyPlaylists, createPlaylist,
  deletePlaylist, loadPlaylistDetail, addSongToPlaylist, removeSongFromPlaylist,
  onSearchInput, loadLibrarySongs, apiFetch
} from '../api'
import type { PlaylistDetail, PlaylistSong } from '../api/playlists'

onMounted(() => {
  loadLibrarySongs(false)
  if (store.deviceUser) {
    loadMyPlaylists()
  }
})

const fileInput = ref<HTMLInputElement | null>(null)
const selectedFile = ref<File | null>(null)
const isAdmin = computed(() => store.deviceUser?.role === 'admin')

function handleFileSelect(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (file) {
    selectedFile.value = file
    store.uploadFile = file
    store.uploadFileName = file.name
    store.uploadStatus = ''
    store.uploadStatusType = ''
  }
}

async function handleUpload() {
  if (!selectedFile.value) {
    store.uploadStatus = '请选择文件'
    store.uploadStatusType = 'error'
    return
  }
  if (selectedFile.value.size > 100 * 1024 * 1024) {
    store.uploadStatus = '文件大小超过 100MB 限制'
    store.uploadStatusType = 'error'
    return
  }

  const formData = new FormData()
  formData.append('file', selectedFile.value)
  store.uploadStatus = '上传中...'
  store.uploadStatusType = 'info'

  try {
    const res = await apiFetch('/api/admin/upload', {
      method: 'POST',
      body: formData
    })
    const data = await res.json()
    if (!data.success) {
      store.uploadStatus = data.error || '上传失败'
      store.uploadStatusType = 'error'
      return
    }

    store.uploadStatus = data.data || '上传成功'
    store.uploadStatusType = 'success'
    selectedFile.value = null
    store.uploadFile = null
    store.uploadFileName = ''
    if (fileInput.value) fileInput.value.value = ''
    onSearchInput()
  } catch {
    store.uploadStatus = '上传失败'
    store.uploadStatusType = 'error'
  }
}

// ─── Playlist detail dialog ──────────────────────────────
const showPlaylistDialog = ref(false)
const currentPlaylist = ref<PlaylistDetail | null>(null)
const playlistLoading = ref(false)

async function openPlaylistDetail(pl: { id: number; name: string }) {
  showPlaylistDialog.value = true
  playlistLoading.value = true
  currentPlaylist.value = await loadPlaylistDetail(pl.id)
  playlistLoading.value = false
}

async function handleRemoveFromPlaylist(songId: number) {
  if (!currentPlaylist.value) return
  const ok = await removeSongFromPlaylist(currentPlaylist.value.id, songId)
  if (ok) {
    currentPlaylist.value = await loadPlaylistDetail(currentPlaylist.value.id)
    await loadMyPlaylists()
  }
}

async function handlePlayAll() {
  if (!currentPlaylist.value?.songs.length) return
  for (const s of currentPlaylist.value.songs) {
    await addToQueue(s.id)
  }
}

// ─── Add-to-playlist dialog ──────────────────────────────
const showAddToPlaylistDialog = ref(false)
const selectedSongId = ref<number | null>(null)

const hasMoreSongs = computed(() => store.searchResults.length < store.searchTotal)
const libraryTitle = computed(() => store.searchQuery.trim() ? '搜索结果' : '电台曲库')
const librarySubtitle = computed(() => store.searchQuery.trim() ? '匹配歌曲' : '公开歌曲')

function openAddToPlaylist(songId: number) {
  selectedSongId.value = songId
  showAddToPlaylistDialog.value = true
}

async function handleAddToPlaylist(playlistId: number) {
  if (!selectedSongId.value) return
  await addSongToPlaylist(playlistId, selectedSongId.value)
  showAddToPlaylistDialog.value = false
  selectedSongId.value = null
}
</script>

<template>
  <div class="am-library">
    <!-- Page Header -->
    <div class="am-page-header mb-5">
      <h1 class="text-h5 font-weight-bold">{{ libraryTitle }}</h1>
      <p class="text-body-2 text-medium-emphasis mt-1">{{ librarySubtitle }} · 共 {{ store.searchTotal }} 首</p>
    </div>

    <!-- Search -->
    <v-card class="mb-5 am-card" elevation="0">
      <v-card-text class="pa-4">
        <v-text-field
          v-model="store.searchQuery"
          placeholder="搜索歌曲或艺术家..."
          prepend-inner-icon="mdi-magnify"
          clearable
          hide-details
          density="comfortable"
          @update:model-value="debouncedSearch"
        />
      </v-card-text>
    </v-card>

    <!-- Song List -->
    <v-card class="mb-5 am-card" elevation="0">
      <v-card-text class="pa-0">
        <div v-if="!store.searchLoading && store.searchResults.length === 0" class="text-center py-12 text-medium-emphasis">
          <v-icon size="48" color="disabled" class="mb-3">mdi-music-note-outline</v-icon>
          <div class="text-body-2">{{ store.searchQuery.trim() ? '未找到匹配的歌曲' : '曲库暂时为空' }}</div>
        </div>
        <v-list v-else lines="three" class="am-song-list">
          <v-list-item
            v-for="(s, index) in store.searchResults"
            :key="s.id"
            class="am-song-item"
            :style="{ '--am-stagger': Math.min(index, 24) * 30 + 'ms' }"
            :title="s.title"
            :subtitle="(s.artist || '') + ' · ' + (s.album || '') + ' · ' + formatTime(s.duration_ms)"
          >
            <template #prepend>
              <div class="am-song-index text-caption text-medium-emphasis font-weight-semibold">
                {{ String(index + 1).padStart(2, '0') }}
              </div>
            </template>
            <template #append>
              <div class="d-flex align-center gap-2">
                <v-btn
                  icon
                  variant="tonal"
                  color="primary"
                  size="small"
                  title="投喂到电台"
                  @click="addToQueue(s.id)"
                >
                  <v-icon size="18">mdi-radio-tower</v-icon>
                </v-btn>
                <v-btn
                  v-if="store.deviceUser && store.myPlaylists.length > 0"
                  icon
                  variant="tonal"
                  color="secondary"
                  size="small"
                  title="添加到歌单"
                  @click="openAddToPlaylist(s.id)"
                >
                  <v-icon size="18">mdi-playlist-plus</v-icon>
                </v-btn>
                <v-btn
                  icon
                  variant="text"
                  color="medium-emphasis"
                  size="small"
                  title="下载"
                  @click="downloadSong(s.id)"
                >
                  <v-icon size="18">mdi-download-outline</v-icon>
                </v-btn>
              </div>
            </template>
          </v-list-item>
        </v-list>
        <div v-if="hasMoreSongs" class="pa-4 pt-2">
          <v-btn
            variant="tonal"
            color="primary"
            block
            rounded="xl"
            :loading="store.searchLoading"
            @click="loadLibrarySongs(true)"
          >
            加载更多
          </v-btn>
        </div>
      </v-card-text>
    </v-card>

    <!-- Two column layout for desktop -->
    <div class="am-library-grid" :class="{ desktop: store.isDesktop }">
      <!-- Left: Upload + Playlists -->
      <div class="am-library-left">
        <!-- Upload -->
        <v-card v-if="isAdmin" class="mb-5 am-card" elevation="0">
          <v-card-title class="text-subtitle-1 font-weight-bold pa-4 pb-2">
            上传歌曲
          </v-card-title>
          <v-card-text class="pa-4 pt-0">
            <input
              ref="fileInput"
              type="file"
              accept=".mp3,.wav,.flac,.ogg,.m4a,.aac"
              style="display: none"
              @change="handleFileSelect"
            >
            <v-btn
              variant="outlined"
              color="primary"
              prepend-icon="mdi-cloud-upload-outline"
              block
              rounded="xl"
              @click="fileInput?.click()"
            >
              {{ selectedFile ? selectedFile.name : '选择文件' }}
            </v-btn>
            <v-btn
              v-if="selectedFile"
              class="mt-3"
              color="primary"
              block
              rounded="xl"
              @click="handleUpload"
            >
              上传
            </v-btn>
            <v-alert
              v-if="store.uploadStatus"
              :type="store.uploadStatusType as any"
              class="mt-3"
              density="compact"
              variant="tonal"
              rounded="lg"
            >
              {{ store.uploadStatus }}
            </v-alert>
          </v-card-text>
        </v-card>

        <!-- Playlists -->
        <v-card v-if="store.deviceUser" class="mb-5 am-card" elevation="0">
          <v-card-title class="text-subtitle-1 font-weight-bold d-flex align-center pa-4 pb-2">
            我的歌单
            <v-spacer />
            <v-chip size="small" color="primary" variant="tonal" rounded="lg">
              {{ store.myPlaylists.length }}
            </v-chip>
          </v-card-title>
          <v-card-text class="pa-4 pt-0">
            <v-list v-if="store.myPlaylists.length > 0" density="compact">
              <v-list-item
                v-for="pl in store.myPlaylists"
                :key="pl.id"
                :title="pl.name"
                :subtitle="(pl.song_count || 0) + ' 首'"
                @click="openPlaylistDetail(pl)"
                class="am-playlist-item"
              >
                <template #prepend>
                  <v-icon color="primary" size="20">mdi-playlist-music-outline</v-icon>
                </template>
                <template #append>
                  <v-btn
                    icon
                    variant="text"
                    color="error"
                    size="x-small"
                    @click.stop="deletePlaylist(pl.id)"
                  >
                    <v-icon size="16">mdi-delete-outline</v-icon>
                  </v-btn>
                </template>
              </v-list-item>
            </v-list>
            <div v-else class="text-center py-6 text-medium-emphasis">
              <v-icon size="32" color="disabled" class="mb-2">mdi-playlist-music-outline</v-icon>
              <div class="text-caption">还没有歌单</div>
            </div>
            <div class="d-flex gap-2 mt-3">
              <v-text-field
                v-model="store.newPlaylistName"
                placeholder="新建歌单名称"
                density="compact"
                hide-details
                rounded="lg"
              />
              <v-btn color="primary" rounded="xl" @click="createPlaylist">创建</v-btn>
            </div>
          </v-card-text>
        </v-card>
      </div>

      <!-- Right: Import info -->
      <div class="am-library-right">
        <v-card class="mb-5 am-card" elevation="0">
          <v-card-title class="text-subtitle-1 font-weight-bold pa-4 pb-2">
            音乐导入
          </v-card-title>
          <v-card-text class="pa-4 pt-0">
            <div class="am-import-info text-body-2 text-medium-emphasis">
              <v-icon size="24" color="primary" class="mr-2">mdi-information-outline</v-icon>
              网易云和远端导入会向服务器新增音乐文件，仅管理员可在后台操作。普通用户仍可下载曲库中已有歌曲。
            </div>
          </v-card-text>
        </v-card>
      </div>
    </div>

    <!-- Playlist Detail Dialog -->
    <v-dialog v-model="showPlaylistDialog" max-width="600">
      <v-card rounded="xl">
        <v-card-title class="text-h6 font-weight-bold d-flex align-center pa-5">
          {{ currentPlaylist?.name || '歌单详情' }}
          <v-spacer />
          <v-btn
            v-if="currentPlaylist && currentPlaylist.songs.length > 0"
            color="primary"
            size="small"
            prepend-icon="mdi-play"
            rounded="xl"
            @click="handlePlayAll"
          >
            播放全部
          </v-btn>
        </v-card-title>
        <v-card-text class="px-5 pb-2">
          <v-progress-linear v-if="playlistLoading" indeterminate color="primary" rounded />
          <div v-else-if="!currentPlaylist || currentPlaylist.songs.length === 0" class="text-center py-8 text-medium-emphasis">
            歌单为空
          </div>
          <v-list v-else lines="two">
            <v-list-item
              v-for="s in currentPlaylist.songs"
              :key="s.id"
              :title="s.title"
              :subtitle="(s.artist || '') + ' · ' + formatTime(s.duration_ms)"
            >
              <template #append>
                <v-btn
                  icon
                  variant="tonal"
                  color="primary"
                  size="small"
                  title="点歌"
                  @click="addToQueue(s.id)"
                >
                  <v-icon size="18">mdi-radio-tower</v-icon>
                </v-btn>
                <v-btn
                  icon
                  variant="text"
                  color="error"
                  size="small"
                  title="从歌单移除"
                  @click="handleRemoveFromPlaylist(s.id)"
                >
                  <v-icon size="18">mdi-playlist-remove</v-icon>
                </v-btn>
              </template>
            </v-list-item>
          </v-list>
        </v-card-text>
        <v-card-actions class="pa-5 pt-2">
          <v-spacer />
          <v-btn variant="text" @click="showPlaylistDialog = false">关闭</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Add to Playlist Dialog -->
    <v-dialog v-model="showAddToPlaylistDialog" max-width="400">
      <v-card rounded="xl">
        <v-card-title class="pa-5 pb-3">添加到歌单</v-card-title>
        <v-card-text class="px-5 pb-2">
          <v-list density="compact">
            <v-list-item
              v-for="pl in store.myPlaylists"
              :key="pl.id"
              :title="pl.name"
              :subtitle="(pl.song_count || 0) + ' 首'"
              @click="handleAddToPlaylist(pl.id)"
              class="rounded-lg"
            >
              <template #prepend>
                <v-icon color="primary" size="20">mdi-playlist-music-outline</v-icon>
              </template>
            </v-list-item>
          </v-list>
        </v-card-text>
        <v-card-actions class="pa-5 pt-2">
          <v-spacer />
          <v-btn variant="text" @click="showAddToPlaylistDialog = false">取消</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.am-library {
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

.am-song-list {
  padding: 8px 0;
}

.am-song-item {
  padding: 4px 16px;
  animation: am-song-enter 0.5s var(--am-ease-emphasized) both;
  animation-delay: var(--am-stagger);
  transform-origin: 50% 65%;
  transition:
    background-color 0.24s var(--am-ease-emphasized),
    transform 0.24s var(--am-ease-spring);
}

.am-song-item:hover {
  background: var(--am-surface-2);
  transform: translateX(2px);
}

.am-song-index {
  width: 28px;
  text-align: center;
  margin-right: 8px;
}

.am-playlist-item {
  cursor: pointer;
  border-radius: var(--am-radius-sm);
}

.am-playlist-item:hover {
  background: var(--am-surface-2);
}

.am-import-info {
  display: flex;
  align-items: flex-start;
  padding: 12px 16px;
  background: var(--am-surface-2);
  border-radius: var(--am-radius-sm);
}

.am-library-grid {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.am-library-grid.desktop {
  flex-direction: row;
  gap: 20px;
}

.am-library-grid.desktop .am-library-left {
  flex: 1;
}

.am-library-grid.desktop .am-library-right {
  flex: 1;
}

.gap-2 {
  gap: 8px;
}

@keyframes am-song-enter {
  from {
    opacity: 0;
    transform: translateY(16px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes slideUp {
  from { transform: translateY(16px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}

@media (prefers-reduced-motion: reduce) {
  .am-song-item {
    animation: none;
    transition: none;
  }
  .am-song-item:hover {
    transform: none;
  }
}
</style>
