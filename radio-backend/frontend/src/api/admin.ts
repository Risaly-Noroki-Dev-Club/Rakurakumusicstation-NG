import { apiBase } from './client'
import { toast } from '../store'

export async function adminSkipNext(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/admin/playlist/next', { method: 'POST' })
    const data = await res.json()
    if (data.success) {
      toast('已切到下一首', 'success')
    } else {
      toast(data.error || '切歌失败', 'error')
    }
  } catch {
    toast('请求失败', 'error')
  }
}

export async function adminSkipPrev(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/admin/playlist/prev', { method: 'POST' })
    const data = await res.json()
    if (data.success) {
      toast('已切到上一首', 'success')
    } else {
      toast(data.error || '切歌失败', 'error')
    }
  } catch {
    toast('请求失败', 'error')
  }
}
