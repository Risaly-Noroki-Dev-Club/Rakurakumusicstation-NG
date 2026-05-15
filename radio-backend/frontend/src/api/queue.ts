import { apiFetch } from './client'
import { store, toast } from '../store'

export async function refreshQueue(): Promise<void> {
  try {
    const res = await apiFetch('/api/queue')
    const data = await res.json()
    if (!data.success) return
    store.queue = data.data || []
    await refreshHistory()
  } catch { /* ignore */ }
}

export async function refreshHistory(): Promise<void> {
  try {
    const res = await apiFetch('/api/queue/history')
    const data = await res.json()
    if (data.success) store.history = (data.data || []).slice(0, 20)
  } catch { /* ignore */ }
}

export async function removeQueueItem(id: number): Promise<void> {
  try {
    await apiFetch('/api/queue/' + id, { method: 'DELETE' })
    refreshQueue()
  } catch { toast('移除失败', 'error') }
}

export async function addToQueue(songId: number): Promise<void> {
  try {
    const res = await apiFetch('/api/queue', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ song_id: songId })
    })
    const data = await res.json()
    if (data.success) {
      toast('已添加到电台队列！', 'success')
      refreshQueue()
    } else {
      toast(data.error || '点歌失败', 'error')
    }
  } catch { toast('请求失败', 'error') }
}
