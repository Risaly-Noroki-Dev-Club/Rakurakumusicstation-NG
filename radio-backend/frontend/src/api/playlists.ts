import { apiUrl } from './client'
import { store, toast } from '../store'

export async function loadMyPlaylists(): Promise<void> {
  try {
    const res = await fetch(apiUrl('/api/playlists'))
    const data = await res.json()
    if (data.success) store.myPlaylists = data.data || []
  } catch { /* ignore */ }
}

export async function createPlaylist(): Promise<void> {
  if (!store.newPlaylistName.trim()) return
  try {
    const res = await fetch(apiUrl('/api/playlists'), {
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

export async function deletePlaylist(id: number): Promise<void> {
  if (!confirm('确定删除此歌单？')) return
  try {
    const res = await fetch(apiUrl('/api/playlists/' + id), { method: 'DELETE' })
    const data = await res.json()
    if (data.success) {
      toast('歌单已删除', 'success')
      await loadMyPlaylists()
    } else {
      toast(data.error || '删除失败', 'error')
    }
  } catch { toast('请求失败', 'error') }
}

export interface PlaylistSong {
  id: number
  title: string
  artist: string
  album?: string
  duration_ms?: number
}

export interface PlaylistDetail {
  id: number
  name: string
  songs: PlaylistSong[]
  song_count: number
  created_at: string
}

export async function loadPlaylistDetail(id: number): Promise<PlaylistDetail | null> {
  try {
    const res = await fetch(apiUrl('/api/playlists/' + id))
    const data = await res.json()
    if (data.success) return data.data
  } catch { /* ignore */ }
  return null
}

export async function addSongToPlaylist(playlistId: number, songId: number): Promise<boolean> {
  try {
    const res = await fetch(apiUrl('/api/playlists/' + playlistId + '/songs'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ song_id: songId })
    })
    const data = await res.json()
    if (data.success) {
      toast('已添加到歌单', 'success')
      return true
    } else {
      toast(data.error || '添加失败', 'error')
    }
  } catch { toast('请求失败', 'error') }
  return false
}

export async function removeSongFromPlaylist(playlistId: number, songId: number): Promise<boolean> {
  try {
    const res = await fetch(apiUrl('/api/playlists/' + playlistId + '/songs'), {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ song_id: songId })
    })
    const data = await res.json()
    if (data.success) {
      toast('已从歌单移除', 'success')
      return true
    } else {
      toast(data.error || '移除失败', 'error')
    }
  } catch { toast('请求失败', 'error') }
  return false
}
