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
import LtPageShell from '../components/lt/LtPageShell.vue'

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
  <!-- 桌面端：LT 风格 -->
  <LtPageShell v-if="store.isDesktop" :title="libraryTitle" :subtitle="librarySubtitle + ' · 共 ' + store.searchTotal + ' 首'">
    <!-- 搜索 -->
    <div class="lt-card lt-search-bar">
      <input
        v-model="store.searchQuery"
        placeholder="搜索歌曲或艺术家..."
        class="lt-input"
        @input="debouncedSearch"
      />
    </div>

    <!-- 歌曲列表 -->
    <div class="lt-card lt-song-list">
      <div v-if="!store.searchLoading && store.searchResults.length === 0" class="lt-empty">
        {{ store.searchQuery.trim() ? '未找到匹配的歌曲' : '曲库暂时为空' }}
      </div>
      <div v-for="(s, index) in store.searchResults" :key="s.id" class="lt-song-item">
        <span class="lt-song-index">{{ String(index + 1).padStart(2, '0') }}</span>
        <div class="lt-song-info">
          <div class="lt-song-title">{{ s.title }}</div>
          <div class="lt-song-artist">{{ (s.artist || '') + ' · ' + (s.album || '') + ' · ' + formatTime(s.duration_ms) }}</div>
        </div>
        <div class="lt-song-actions">
          <button class="lt-icon-btn" title="投喂到电台" @click="addToQueue(s.id)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0M12 20h.01"/></svg>
          </button>
          <button v-if="store.deviceUser && store.myPlaylists.length > 0" class="lt-icon-btn" title="添加到歌单" @click="openAddToPlaylist(s.id)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
          </button>
          <button class="lt-icon-btn" title="下载" @click="downloadSong(s.id)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>
          </button>
        </div>
      </div>
      <button v-if="hasMoreSongs" class="lt-load-more" :disabled="store.searchLoading" @click="loadLibrarySongs(true)">
        {{ store.searchLoading ? '加载中...' : '加载更多' }}
      </button>
    </div>

    <!-- 上传 + 歌单 + 导入 -->
    <div class="lt-lib-grid">
      <!-- 上传 -->
      <div v-if="isAdmin" class="lt-card">
        <div class="lt-card-title">上传歌曲</div>
        <input ref="fileInput" type="file" accept=".mp3,.wav,.flac,.ogg,.m4a,.aac" style="display:none" @change="handleFileSelect">
        <button class="lt-btn-outline" @click="fileInput?.click()">
          {{ selectedFile ? selectedFile.name : '选择文件' }}
        </button>
        <button v-if="selectedFile" class="lt-btn-primary mt" @click="handleUpload">上传</button>
        <div v-if="store.uploadStatus" class="lt-alert" :class="store.uploadStatusType">{{ store.uploadStatus }}</div>
      </div>

      <!-- 歌单 -->
      <div v-if="store.deviceUser" class="lt-card">
        <div class="lt-card-title">我的歌单 ({{ store.myPlaylists.length }})</div>
        <div v-if="store.myPlaylists.length === 0" class="lt-empty-sm">还没有歌单</div>
        <div v-for="pl in store.myPlaylists" :key="pl.id" class="lt-playlist-row" @click="openPlaylistDetail(pl)">
          <span class="lt-playlist-name">{{ pl.name }}</span>
          <span class="lt-playlist-count">{{ (pl.song_count || 0) }} 首</span>
          <button class="lt-icon-btn danger" @click.stop="deletePlaylist(pl.id)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
        <div class="lt-create-row">
          <input v-model="store.newPlaylistName" placeholder="新建歌单名称" class="lt-input sm" />
          <button class="lt-btn-primary sm" @click="createPlaylist">创建</button>
        </div>
      </div>

      <!-- 导入说明 -->
      <div class="lt-card">
        <div class="lt-card-title">音乐导入</div>
        <p class="lt-info-text">网易云和远端导入会向服务器新增音乐文件，仅管理员可在后台操作。普通用户仍可下载曲库中已有歌曲。</p>
      </div>
    </div>

    <!-- 歌单详情弹窗 -->
    <div v-if="showPlaylistDialog" class="lt-search-overlay" @click.self="showPlaylistDialog = false">
      <div class="lt-search-modal">
        <div class="lt-search-head">
          <span class="lt-modal-title">{{ currentPlaylist?.name || '歌单详情' }}</span>
          <button v-if="currentPlaylist && currentPlaylist.songs.length > 0" class="lt-btn-primary sm" @click="handlePlayAll">播放全部</button>
          <button class="lt-search-close" @click="showPlaylistDialog = false"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button>
        </div>
        <div class="lt-search-results">
          <div v-if="playlistLoading" class="lt-empty">加载中...</div>
          <div v-else-if="!currentPlaylist || currentPlaylist.songs.length === 0" class="lt-empty">歌单为空</div>
          <div v-for="s in currentPlaylist?.songs" :key="s.id" class="lt-search-item">
            <div class="lt-queue-info">
              <div class="lt-queue-item-title">{{ s.title }}</div>
              <div class="lt-queue-item-artist">{{ (s.artist || '') + ' · ' + formatTime(s.duration_ms) }}</div>
            </div>
            <button class="lt-icon-btn" title="点歌" @click="addToQueue(s.id)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 8v8M8 12h8"/></svg></button>
            <button class="lt-icon-btn danger" title="移除" @click="handleRemoveFromPlaylist(s.id)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button>
          </div>
        </div>
      </div>
    </div>

    <!-- 添加到歌单弹窗 -->
    <div v-if="showAddToPlaylistDialog" class="lt-search-overlay" @click.self="showAddToPlaylistDialog = false">
      <div class="lt-search-modal sm-modal">
        <div class="lt-search-head"><span class="lt-modal-title">添加到歌单</span><button class="lt-search-close" @click="showAddToPlaylistDialog = false"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button></div>
        <div class="lt-search-results">
          <div v-for="pl in store.myPlaylists" :key="pl.id" class="lt-search-item" @click="handleAddToPlaylist(pl.id)">
            <span class="lt-playlist-name">{{ pl.name }}</span>
            <span class="lt-playlist-count">{{ (pl.song_count || 0) }} 首</span>
          </div>
        </div>
      </div>
    </div>
  </LtPageShell>

  <!-- 移动端：原有布局 -->
  <div v-else class="am-library">
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
/* ─── LT 桌面端样式 ─── */
.lt-card {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-md);
  padding: 16px;
  box-shadow: var(--lt-shadow-subtle);
}

.lt-search-bar {
  flex-shrink: 0;
}

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
.lt-input.sm { font-size: 13px; }

.lt-song-list {
  flex: 1;
  overflow-y: auto;
  min-height: 200px;
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
}

.lt-song-item:last-child { border-bottom: none; }

.lt-song-index {
  font-size: 11px;
  font-weight: 600;
  color: var(--lt-text-muted);
  width: 24px;
  text-align: center;
  flex-shrink: 0;
  font-family: 'SF Mono', monospace;
}

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
  gap: 4px;
  flex-shrink: 0;
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

.lt-load-more {
  margin-top: 12px;
  padding: 10px;
  border: 1px solid var(--lt-divider);
  border-radius: var(--lt-radius-sm);
  background: transparent;
  color: var(--lt-text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
  width: 100%;
}

.lt-load-more:hover { background: var(--lt-btn-bg); }
.lt-load-more:disabled { opacity: 0.5; cursor: default; }

.lt-lib-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 16px;
  flex-shrink: 0;
}

.lt-card-title {
  font-family: var(--lt-font-serif);
  font-size: 16px;
  font-weight: 700;
  color: var(--lt-text-primary);
  margin-bottom: 12px;
}

.lt-btn-outline {
  width: 100%;
  padding: 10px 14px;
  border: 1px solid var(--lt-divider);
  border-radius: var(--lt-radius-sm);
  background: transparent;
  color: var(--lt-text-primary);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
  text-align: center;
}

.lt-btn-outline:hover { background: var(--lt-btn-bg); }

.lt-btn-primary {
  padding: 10px 20px;
  border: none;
  border-radius: var(--lt-radius-sm);
  background: var(--lt-play-btn-bg);
  color: var(--lt-play-btn-icon);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
}

.lt-btn-primary:hover { opacity: 0.88; }
.lt-btn-primary.mt { margin-top: 10px; width: 100%; }
.lt-btn-primary.sm { padding: 6px 14px; font-size: 12px; }

.lt-alert {
  margin-top: 10px;
  padding: 8px 12px;
  border-radius: var(--lt-radius-sm);
  font-size: 12px;
  background: var(--lt-btn-bg);
  color: var(--lt-text-secondary);
}

.lt-alert.error { background: rgba(229,57,53,0.12); color: #e53935; }
.lt-alert.success { background: var(--lt-accent-soft); color: var(--lt-accent-text); }

.lt-playlist-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 4px;
  border-bottom: 1px solid var(--lt-divider);
  cursor: pointer;
  transition: background 0.15s;
}

.lt-playlist-row:hover { background: var(--lt-btn-bg); }
.lt-playlist-row:last-child { border-bottom: none; }

.lt-playlist-name {
  flex: 1;
  font-size: 14px;
  font-weight: 500;
  color: var(--lt-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lt-playlist-count {
  font-size: 12px;
  color: var(--lt-text-secondary);
  flex-shrink: 0;
}

.lt-create-row {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.lt-info-text {
  font-size: 13px;
  line-height: 1.5;
  color: var(--lt-text-secondary);
}

.lt-modal-title {
  font-family: var(--lt-font-serif);
  font-size: 18px;
  font-weight: 700;
  color: var(--lt-text-primary);
  flex: 1;
}

.lt-search-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 300;
}

.lt-search-modal {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-lg);
  width: 90%;
  max-width: 460px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: var(--lt-shadow-card);
}

.lt-search-modal.sm-modal { max-width: 360px; }

.lt-search-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--lt-divider);
}

.lt-search-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--lt-text-secondary);
  cursor: pointer;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}

.lt-search-close:hover { background: var(--lt-btn-bg); }
.lt-search-close svg { width: 18px; height: 18px; }

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
  transition: background 0.15s;
}

.lt-search-item:hover { background: var(--lt-btn-bg); }

.lt-queue-info { min-width: 0; flex: 1; }
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
  color: var(--lt-text-secondary);
  margin-top: 1px;
  /* 规则1+2: 缩小字号 + 省略号截断 */
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

/* ─── 移动端原有样式 ─── */
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
