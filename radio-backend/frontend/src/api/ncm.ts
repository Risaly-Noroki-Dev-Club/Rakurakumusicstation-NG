import { apiFetch } from './client'
import { store } from '../store'

export async function loadUserNcmStatus(): Promise<void> {
  try {
    const res = await apiFetch('/api/ncm')
    if (!res.ok) return
    const d = await res.json()
    if (!d.success) return
    const data = d.data
    if (data.configured) {
      const label = 'Cookie 已配置'
      store.userNcmBadge = '✓ ' + label
      store.userNcmBadgeClass = 'ok'
    } else {
      store.userNcmBadge = '未配置（游客模式）'
      store.userNcmBadgeClass = 'none'
    }
  } catch { /* ignore */ }
}

export async function saveUserNcmSettings(): Promise<void> {
  const payload = { cookie: store.userNcmCookie.trim() }
  if (!payload.cookie) {
    store.userNcmResult = '请填写 Cookie'
    store.userNcmResultType = 'error'
    return
  }
  if (!payload.cookie.includes('MUSIC_U=')) {
    store.userNcmResult = 'Cookie 缺少 MUSIC_U'
    store.userNcmResultType = 'error'
    return
  }
  try {
    const res = await apiFetch('/api/ncm', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    const data = await res.json()
    if (data.success) {
      store.userNcmResult = '✅ 保存成功'
      store.userNcmResultType = 'success'
      loadUserNcmStatus()
    } else {
      store.userNcmResult = '❌ ' + (data.error || '保存失败')
      store.userNcmResultType = 'error'
    }
  } catch {
    store.userNcmResult = '❌ 请求失败'
    store.userNcmResultType = 'error'
  }
}

export async function testUserNcmLogin(): Promise<void> {
  store.userNcmResult = '测试中...'
  store.userNcmResultType = 'info'
  try {
    const res = await apiFetch('/api/ncm/test', {
      method: 'POST'
    })
    const data = await res.json()
    if (data.success) {
      const d = data.data
      store.userNcmResult = (d.success ? '✅ ' : '❌ ') + (d.output || (d.success ? '登录成功' : '登录失败'))
      store.userNcmResultType = d.success ? 'success' : 'error'
    } else {
      store.userNcmResult = '❌ 请求失败'
      store.userNcmResultType = 'error'
    }
  } catch {
    store.userNcmResult = '❌ 请求失败'
    store.userNcmResultType = 'error'
  }
}
