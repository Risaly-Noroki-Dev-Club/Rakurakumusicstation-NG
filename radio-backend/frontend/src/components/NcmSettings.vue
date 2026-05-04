<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { store } from '../store'
import { getBackendUrl } from '../api'
import StatusMessage from './StatusMessage.vue'

const props = defineProps<{
  apiPrefix: string
  stateGetter: () => {
    badge: string; badgeClass: string; activeTab: string
    cookie: string; phone: string; password: string
    result: string; resultType: string
  }
  stateSetter: (patch: Record<string, any>) => void
}>()

const activeTab = ref('cookie')
const cookie = ref('')
const phone = ref('')
const password = ref('')
const badge = ref('')
const badgeClass = ref('none')
const result = ref('')
const resultType = ref('')

function syncFromStore() {
  const s = props.stateGetter()
  badge.value = s.badge
  badgeClass.value = s.badgeClass
  activeTab.value = s.activeTab
  cookie.value = s.cookie
  phone.value = s.phone
  password.value = s.password
  result.value = s.result
  resultType.value = s.resultType
}

function syncToStore() {
  props.stateSetter({
    ncmActiveTab: activeTab.value,
    ncmCookie: cookie.value,
    ncmPhone: phone.value,
    ncmPassword: password.value,
  })
}

onMounted(() => { syncFromStore(); loadStatus() })

watch([activeTab, cookie, phone, password], () => syncToStore())

function authHeaders(): Record<string, string> {
  const h: Record<string, string> = {}
  if (store.token) h['Authorization'] = 'Bearer ' + store.token
  return h
}

function showResult(msg: string, type: string) {
  result.value = msg
  resultType.value = type
  props.stateSetter({ ncmResult: msg, ncmResultType: type })
}

async function loadStatus() {
  try {
    const res = await fetch(getBackendUrl() + props.apiPrefix, { headers: authHeaders() })
    const d = await res.json()
    if (!d.success) return
    const data = d.data
    if (data.configured) {
      const label = data.method === 'cookie' ? 'Cookie 已配置' : '手机号 ' + (data.phone_hint || '') + ' 已配置'
      badge.value = '✓ ' + label
      badgeClass.value = 'ok'
    } else {
      badge.value = '未配置（游客模式）'
      badgeClass.value = 'none'
    }
    props.stateSetter({ ncmBadge: badge.value, ncmBadgeClass: badgeClass.value })
  } catch { /* ignore */ }
}

async function saveNcm() {
  const payload = activeTab.value === 'cookie'
    ? { cookie: cookie.value.trim(), phone: '', password: '' }
    : { phone: phone.value.trim(), password: password.value, cookie: '' }
  if (activeTab.value === 'cookie' && !payload.cookie)
    return showResult('请填写 Cookie', 'error')
  if (activeTab.value === 'phone' && (!payload.phone || !payload.password))
    return showResult('请填写手机号和密码', 'error')
  try {
    const res = await fetch(getBackendUrl() + props.apiPrefix, {
      method: 'POST',
      headers: { ...authHeaders(), 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    const data = await res.json()
    if (data.success) { showResult('✅ 保存成功', 'success'); loadStatus() }
    else showResult('❌ ' + (data.error || '保存失败'), 'error')
  } catch { showResult('❌ 请求失败', 'error') }
}

async function testNcm() {
  showResult('测试中...', 'info')
  try {
    const res = await fetch(getBackendUrl() + props.apiPrefix + '/test', {
      method: 'POST', headers: authHeaders()
    })
    const data = await res.json()
    if (data.success) {
      const d = data.data
      showResult(
        (d.success ? '✅ ' : '❌ ') + (d.output || (d.success ? '登录成功' : '登录失败')),
        d.success ? 'success' : 'error'
      )
    } else showResult('❌ 请求失败', 'error')
  } catch { showResult('❌ 请求失败', 'error') }
}
</script>

<template>
  <div class="card admin-panel">
    <h3>🎵 网易云账号配置 <span v-if="badge" :class="'ncm-badge ncm-' + badgeClass">{{ badge }}</span></h3>
    <div class="tab-switch">
      <button :class="{ active: activeTab === 'cookie' }" @click="activeTab = 'cookie'">Cookie 方式</button>
      <button :class="{ active: activeTab === 'phone' }" @click="activeTab = 'phone'">手机号方式</button>
    </div>
    <div v-show="activeTab === 'cookie'">
      <label>Cookie 文本:</label>
      <textarea v-model="cookie" rows="4" placeholder="MUSIC_U=xxx; __csrf=xxx; ..."></textarea>
    </div>
    <div v-show="activeTab === 'phone'">
      <label>手机号:</label>
      <input v-model="phone" placeholder="13800138000">
      <label style="margin-top:8px">密码:</label>
      <input v-model="password" type="password" placeholder="明文密码">
    </div>
    <div style="margin-top:12px;display:flex;gap:8px">
      <button class="btn btn-primary btn-small" @click="saveNcm">💾 保存</button>
      <button class="btn btn-secondary btn-small" @click="testNcm">🔍 测试登录</button>
    </div>
    <StatusMessage :message="result" :type="resultType as any" />
  </div>
</template>

<style scoped>
.ncm-badge {
  font-size: 0.8rem; font-weight: normal; padding: 2px 8px; border-radius: 10px; margin-left: 10px;
}
.ncm-ok { background: rgba(40,167,69,0.15); color: var(--success, #28a745); }
.ncm-none { background: rgba(108,117,125,0.15); color: #6c757d; }
.tab-switch { display: flex; gap: 8px; margin-bottom: 12px; }
.tab-switch button {
  padding: 6px 14px; border: 1px solid var(--border, #ddd); border-radius: 6px; cursor: pointer;
  background: var(--card-bg, #fff); color: var(--text, #333);
}
.tab-switch button.active { background: var(--primary, #764ba2); color: #fff; border-color: var(--primary, #764ba2); }
textarea, input { width: 100%; padding: 8px; border: 1px solid var(--border, #ddd); border-radius: 6px; }
label { display: block; margin-bottom: 4px; font-weight: 600; }
</style>
