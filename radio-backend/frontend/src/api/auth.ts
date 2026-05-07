import { apiBase } from './client'
import { store, toast } from '../store'
import { loadMyPlaylists } from './playlists'
import { loadUserNcmStatus } from './ncm'

export async function loadDeviceUser(): Promise<void> {
  try {
    const res = await fetch(apiBase + '/api/auth/me')
    const data = await res.json()
    if (data.success && data.data) {
      store.deviceUser = {
        id: data.data.id,
        display_name: data.data.display_name || ('Listener-' + String(data.data.id).padStart(4, '0')),
        role: data.data.role,
        device_token: data.data.device_token || ''
      }
      loadMyPlaylists()
      loadUserNcmStatus()
    }
  } catch { /* ignore */ }
}

export async function setDisplayName(name: string): Promise<boolean> {
  try {
    const res = await fetch(apiBase + '/api/auth/name', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ display_name: name })
    })
    const data = await res.json()
    if (data.success) {
      if (store.deviceUser) store.deviceUser.display_name = name
      toast('显示名称已更新', 'success')
      return true
    } else {
      toast(data.error || '更新失败', 'error')
      return false
    }
  } catch {
    toast('请求失败', 'error')
    return false
  }
}

export async function claimAdmin(token: string): Promise<boolean> {
  try {
    const res = await fetch(apiBase + '/api/auth/claim-admin', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ admin_setup_token: token })
    })
    const data = await res.json()
    if (data.success) {
      if (store.deviceUser) store.deviceUser.role = 'admin'
      toast('已获得管理员权限', 'success')
      return true
    } else {
      toast(data.error || '验证失败', 'error')
      return false
    }
  } catch {
    toast('请求失败', 'error')
    return false
  }
}
