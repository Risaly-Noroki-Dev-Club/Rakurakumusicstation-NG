// 聚合导出：所有 API 模块通过此文件统一暴露，保持向后兼容。

export { apiBase, getStreamUrl, getAudioEngineUrl, getBackendUrl } from './api/client'

export { loadStationInfo } from './api/station'

export { loadDeviceUser, setDisplayName, claimAdmin } from './api/auth'

export { refreshQueue, refreshHistory, removeQueueItem, addToQueue } from './api/queue'

export { onSearchInput, downloadSong, uploadSong } from './api/songs'

export { loadMyPlaylists, createPlaylist, deletePlaylist, loadPlaylistDetail, addSongToPlaylist, removeSongFromPlaylist } from './api/playlists'

export { adminSkipNext, adminSkipPrev } from './api/admin'

export { loadUserNcmStatus, saveUserNcmSettings, testUserNcmLogin } from './api/ncm'

export {
  connectWebSocket,
  startPollers,
  stopPollers,
  refreshPlaybackPoll,
  debouncedSearch,
  volumeDown,
  volumeUp,
  getWs,
} from './api/websocket'
