<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { store } from '../store'
import { apiFetch } from '../api'

const props = defineProps<{
  apiPrefix?: string
  userMode?: boolean
}>()

const prefix = props.apiPrefix || '/api/ncm'
const isUser = props.userMode || false

const activeTab = ref('cookie')
const cookie = ref('')
const badge = ref('')
const badgeClass = ref('none')
const result = ref('')
const resultType = ref('')

function getStorePrefix() {
  return isUser ? 'userNcm' : 'ncm'
}

function syncFromStore() {
  const p = getStorePrefix()
  activeTab.value = (store as any)[p + 'ActiveTab'] || 'cookie'
  cookie.value = (store as any)[p + 'Cookie'] || ''
  badge.value = (store as any)[p + 'Badge'] || '未配置'
  badgeClass.value = (store as any)[p + 'BadgeClass'] || 'none'
  result.value = (store as any)[p + 'Result'] || ''
  resultType.value = (store as any)[p + 'ResultType'] || ''
}

function syncToStore() {
  const p = getStorePrefix()
  ;(store as any)[p + 'ActiveTab'] = activeTab.value
  ;(store as any)[p + 'Cookie'] = cookie.value
}

onMounted(() => { syncFromStore(); loadStatus() })

function showResult(msg: string, type: string) {
  result.value = msg
  resultType.value = type
  const p = getStorePrefix()
  ;(store as any)[p + 'Result'] = msg
  ;(store as any)[p + 'ResultType'] = type
}

async function loadStatus() {
  try {
    const res = await apiFetch(prefix)
    if (!res.ok) return
    const d = await res.json()
    if (!d.success) return
    const data = d.data
    if (data.configured) {
      const label = 'Cookie 已配置'
      badge.value = label
      badgeClass.value = 'ok'
    } else {
      badge.value = isUser ? '未配置（游客模式）' : '未配置'
      badgeClass.value = 'none'
    }
    const p = getStorePrefix()
    ;(store as any)[p + 'Badge'] = badge.value
    ;(store as any)[p + 'BadgeClass'] = badgeClass.value
  } catch { /* ignore */ }
}

async function saveNcm() {
  syncToStore()
  const payload = { cookie: cookie.value.trim() }
  if (!payload.cookie)
    return showResult('请填写 Cookie', 'error')
  if (!payload.cookie.includes('MUSIC_U='))
    return showResult('Cookie 缺少 MUSIC_U，请粘贴完整 Request Headers Cookie', 'error')
  try {
    const res = await apiFetch(prefix, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    const data = await res.json()
    if (data.success) { showResult('保存成功', 'success'); loadStatus() }
    else showResult(data.error || '保存失败', 'error')
  } catch { showResult('请求失败', 'error') }
}

async function testNcm() {
  showResult('测试中...', 'info')
  try {
    const res = await apiFetch(prefix + '/test', { method: 'POST' })
    const data = await res.json()
    if (data.success) {
      const d = data.data
      showResult(d.output || (d.success ? '登录成功' : '登录失败'), d.success ? 'success' : 'error')
    } else showResult(data.error || '请求失败', 'error')
  } catch { showResult('请求失败', 'error') }
}
</script>

<template>
  <v-card elevation="1">
    <v-card-title class="text-subtitle-1 font-weight-bold d-flex align-center">
      {{ isUser ? '我的网易云账号' : '网易云账号配置' }}
      <v-spacer />
      <v-chip
        size="small"
        :color="badgeClass === 'ok' ? 'success' : 'default'"
        variant="tonal"
      >
        {{ badge }}
      </v-chip>
    </v-card-title>

    <v-card-text>
      <p v-if="isUser" class="text-body-2 text-medium-emphasis mb-4">
        登录后可下载 VIP 歌曲。请粘贴完整 Request Headers Cookie，不要只填 MUSIC_U。
      </p>

      <v-textarea
        v-model="cookie"
        rows="5"
        placeholder="粘贴完整网易云 Cookie 字符串，需包含 MUSIC_U=..."
        hide-details
      />
      <p class="text-caption text-medium-emphasis mt-2">
        获取方式：浏览器打开 <b>music.163.com</b> 并登录 → F12 → Network → 任意请求 → Request Headers → Cookie
      </p>

      <div class="d-flex gap-3 mt-4">
        <v-btn color="primary" prepend-icon="mdi-content-save" @click="saveNcm">
          保存
        </v-btn>
        <v-btn variant="outlined" prepend-icon="mdi-connection" @click="testNcm">
          测试连接
        </v-btn>
      </div>

      <v-alert
        v-if="result"
        :type="resultType as any"
        class="mt-4"
        density="compact"
        variant="tonal"
      >
        {{ result }}
      </v-alert>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.gap-3 {
  gap: 12px;
}
</style>
