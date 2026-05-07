import { apiBase } from './client'
import { store, toast } from '../store'

export async function loadMyPlaylists(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/playlists')
    const data = await res.json()
    if (data.success) store.myPlaylists = data.data || []
  } catch { /* ignore */ }
}

export async function createPlaylist(): Promise<void> {
  if (!store.newPlaylistName.trim()) return
  try {
    const res = await fetch(apiBase + '/api/playlists', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: store.newPlaylistName.trim() })
    })
    const data = await res.json()
    if (data && data.success) {
      toast('歌单创建成功', 'success')
      store.newPlaylistName = ''
      await loadMyPlaylists()
    } else {
      toast((data && data.error) || '创建失败', 'error')
    }
  } catch { toast('请求失败', 'error') }
}
