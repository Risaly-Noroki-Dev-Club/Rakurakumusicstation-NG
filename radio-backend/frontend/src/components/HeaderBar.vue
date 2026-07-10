<script setup lang="ts">
import { computed, ref } from 'vue'
import { Monitor, Sun, Moon, Music, KeyRound } from '@lucide/vue'
import { store, cycleTheme, THEMES, toast } from '../store'
import { setDisplayName, claimAdmin } from '../api'

const themeIcon = computed(() => {
  const iconMap: Record<string, any> = { auto: Monitor, light: Sun, dark: Moon }
  return iconMap[THEMES[store.themeIdx]] || Monitor
})

const displayName = computed(() => store.deviceUser?.display_name || '')

const showNameEditor = ref(false)
const editName = ref('')
const showAdminClaim = ref(false)
const adminToken = ref('')

function openNameEditor() {
  editName.value = displayName.value
  showNameEditor.value = true
}

async function saveName() {
  if (!editName.value.trim()) {
    toast('名称不能为空', 'error')
    return
  }
  if (await setDisplayName(editName.value.trim())) {
    showNameEditor.value = false
  }
}

function cancelNameEdit() {
  showNameEditor.value = false
}

function openAdminClaim() {
  adminToken.value = ''
  showAdminClaim.value = true
}

async function doClaimAdmin() {
  if (!adminToken.value.trim()) {
    toast('请输入管理员设置令牌', 'error')
    return
  }
  if (await claimAdmin(adminToken.value.trim())) {
    showAdminClaim.value = false
  }
}

function cancelAdminClaim() {
  showAdminClaim.value = false
}
</script>

<template>
  <header>
    <h1><Music :size="16" /><span>{{ store.stationName }}</span></h1>
    <div>
      <button class="theme-toggle" title="切换主题" @click="cycleTheme"><component :is="themeIcon" :size="18" /></button>
      <span v-if="displayName" class="user-display" @click="openNameEditor" title="点击修改名称">{{ displayName }}</span>
      <span v-if="!displayName" class="user-display">连接中...</span>
      <button v-if="store.deviceUser && store.deviceUser.role !== 'admin'" class="btn btn-secondary btn-small" @click="openAdminClaim" title="使用设置令牌获取管理员权限"><KeyRound :size="16" /></button>
    </div>
  </header>

  <!-- Name editor modal -->
  <div v-if="showNameEditor" class="setup-overlay">
    <div class="card" style="width:360px;max-width:90vw">
      <h3>设置显示名称</h3>
      <div class="form-group">
        <input v-model="editName" placeholder="输入名称" @keyup.enter="saveName" maxlength="32" autofocus>
      </div>
      <div style="display:flex;gap:8px;margin-top:12px">
        <button class="btn btn-primary" @click="saveName" style="flex:1">保存</button>
        <button class="btn btn-secondary" @click="cancelNameEdit" style="flex:1">取消</button>
      </div>
    </div>
  </div>

  <!-- Admin claim modal -->
  <div v-if="showAdminClaim" class="setup-overlay">
    <div class="card" style="width:360px;max-width:90vw">
      <h3>申请管理员权限</h3>
      <p style="color:var(--text-muted);margin-bottom:12px">输入 config.toml 中设置的管理员令牌</p>
      <div class="form-group">
        <input v-model="adminToken" placeholder="管理员设置令牌" @keyup.enter="doClaimAdmin" autofocus>
      </div>
      <div style="display:flex;gap:8px;margin-top:12px">
        <button class="btn btn-primary" @click="doClaimAdmin" style="flex:1">提交</button>
        <button class="btn btn-secondary" @click="cancelAdminClaim" style="flex:1">取消</button>
      </div>
    </div>
  </div>
</template>
