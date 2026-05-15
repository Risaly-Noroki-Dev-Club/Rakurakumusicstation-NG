import { apiFetch } from './client'
import { store, toast } from '../store'

const SONG_PAGE_SIZE = 50

export async function loadLibrarySongs(append = false): Promise<void> {
  const q = store.searchQuery.trim()
  const offset = append ? store.searchOffset : 0
  if (store.searchLoading) return
  store.searchLoading = true
  try {
    const res = await apiFetch(
      '/api/songs?q=' + encodeURIComponent(q) +
      '&limit=' + SONG_PAGE_SIZE +
      '&offset=' + offset
    )
    const data = await res.json()
    if (data.success) {
      const page = data.data
      const songs = page && page.data ? page.data : []
      store.searchResults = append ? store.searchResults.concat(songs) : songs
      store.searchTotal = page?.total || 0
      store.searchOffset = offset + songs.length
    }
  } catch { /* ignore */ }
  finally {
    store.searchLoading = false
  }
}

export async function onSearchInput(): Promise<void> {
  await loadLibrarySongs(false)
}

export async function downloadSong(songId: number): Promise<void> {
  try {
    const res = await apiFetch('/api/songs/' + songId + '/download')
    if (!res.ok) {
      const data = await res.json().catch(() => ({ error: '下载失败' }))
      toast(data.error || '下载失败', 'error')
      return
    }
    const blob = await res.blob()
    const disposition = res.headers.get('Content-Disposition') || ''
    const match = disposition.match(/filename="(.+)"/)
    const filename = match ? match[1] : 'song_' + songId + '.mp3'
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    toast('下载完成', 'success')
  } catch { toast('下载失败', 'error') }
}

export async function uploadSong(file: File): Promise<boolean> {
  if (file.size > 100 * 1024 * 1024) {
    toast('文件大小超过 100MB 限制', 'error')
    return false
  }
  const formData = new FormData()
  formData.append('file', file)
  try {
    const res = await apiFetch('/api/songs/upload', {
      method: 'POST',
      body: formData
    })
    const data = await res.json()
    if (data.success) {
      toast(data.data || '上传成功', 'success')
      return true
    } else {
      toast(data.error || '上传失败', 'error')
      return false
    }
  } catch {
    toast('上传失败', 'error')
    return false
  }
}
