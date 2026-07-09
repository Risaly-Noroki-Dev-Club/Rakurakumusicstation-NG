<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { store, cycleTheme, THEMES, toast } from '../store'
import { setDisplayName, claimAdmin } from '../api'
import LtPageShell from '../components/lt/LtPageShell.vue'

const router = useRouter()

const showNameEditor = ref(false)
const editName = ref('')
const showAdminClaim = ref(false)
const adminToken = ref('')

function openNameEditor() {
  editName.value = store.deviceUser?.display_name || ''
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

async function doClaimAdmin() {
  if (!adminToken.value.trim()) {
    toast('请输入管理员设置令牌', 'error')
    return
  }
  if (await claimAdmin(adminToken.value.trim())) {
    showAdminClaim.value = false
  }
}

const themeLabels: Record<string, string> = {
  auto: '跟随系统',
  dark: '深色',
  light: '浅色',
}

const currentThemeLabel = computed(() => {
  return themeLabels[THEMES[store.themeIdx]] || '跟随系统'
})

const themeIcons: Record<string, string> = {
  auto: 'mdi-theme-light-dark',
  dark: 'mdi-weather-night',
  light: 'mdi-white-balance-sunny',
}

const currentThemeIcon = computed(() => {
  return themeIcons[THEMES[store.themeIdx]] || 'mdi-theme-light-dark'
})

// 头像渐变色
function avatarGradient(name: string): string {
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash)
  const h1 = Math.abs(hash) % 360
  const h2 = (h1 + 40) % 360
  return `linear-gradient(135deg, hsl(${h1}, 70%, 60%), hsl(${h2}, 70%, 55%))`
}

const userName = computed(() => store.deviceUser?.display_name || 'Guest')
</script>

<template>
  <!-- 桌面端：LT 风格 -->
  <LtPageShell v-if="store.isDesktop" title="Account" subtitle="管理你的账户和偏好">
    <!-- 用户信息 -->
    <div class="lt-card lt-user-card">
      <div class="lt-user-row">
        <div class="lt-user-avatar-lg" :style="{ background: avatarGradient(userName) }">{{ userName.charAt(0).toUpperCase() }}</div>
        <div class="lt-user-meta">
          <div class="lt-user-name-lg">{{ store.deviceUser?.display_name || '未登录' }}</div>
          <span class="lt-chip" :class="{ admin: store.deviceUser?.role === 'admin' }">{{ store.deviceUser?.role === 'admin' ? '管理员' : '用户' }}</span>
        </div>
      </div>
      <button class="lt-btn-outline" @click="openNameEditor">修改名称</button>
    </div>

    <!-- 外观 -->
    <div class="lt-card">
      <div class="lt-card-title">外观</div>
      <div class="lt-setting-row" @click="cycleTheme">
        <div class="lt-setting-icon">
          <v-icon :icon="currentThemeIcon" size="20" />
        </div>
        <div class="lt-setting-info">
          <div class="lt-setting-label">主题</div>
          <div class="lt-setting-desc">{{ currentThemeLabel }}</div>
        </div>
        <svg class="lt-chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="9 18 15 12 9 6"/></svg>
      </div>
    </div>

    <!-- 管理员 -->
    <div v-if="store.deviceUser?.role !== 'admin'" class="lt-card">
      <div class="lt-card-title">管理员</div>
      <p class="lt-info-text">输入 config.toml 中设置的管理员令牌以获取管理员权限。</p>
      <button class="lt-btn-primary" @click="showAdminClaim = true">申请管理员权限</button>
    </div>
    <div v-else class="lt-card">
      <div class="lt-card-title">管理员</div>
      <button class="lt-btn-primary" @click="router.push('/admin')">进入管理后台</button>
    </div>

    <!-- 名称编辑弹窗 -->
    <div v-if="showNameEditor" class="lt-search-overlay" @click.self="showNameEditor = false">
      <div class="lt-search-modal sm-modal">
        <div class="lt-search-head"><span class="lt-modal-title">设置显示名称</span><button class="lt-search-close" @click="showNameEditor = false"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button></div>
        <div class="lt-modal-body">
          <input v-model="editName" placeholder="输入名称" maxlength="32" class="lt-input" autofocus @keyup.enter="saveName" />
        </div>
        <div class="lt-modal-actions">
          <button class="lt-btn-text" @click="showNameEditor = false">取消</button>
          <button class="lt-btn-primary sm" @click="saveName">保存</button>
        </div>
      </div>
    </div>

    <!-- 管理员申请弹窗 -->
    <div v-if="showAdminClaim" class="lt-search-overlay" @click.self="showAdminClaim = false">
      <div class="lt-search-modal sm-modal">
        <div class="lt-search-head"><span class="lt-modal-title">申请管理员权限</span><button class="lt-search-close" @click="showAdminClaim = false"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button></div>
        <div class="lt-modal-body">
          <p class="lt-info-text mb">输入 config.toml 中设置的管理员令牌</p>
          <input v-model="adminToken" placeholder="管理员设置令牌" type="password" class="lt-input" autofocus @keyup.enter="doClaimAdmin" />
        </div>
        <div class="lt-modal-actions">
          <button class="lt-btn-text" @click="showAdminClaim = false">取消</button>
          <button class="lt-btn-primary sm" @click="doClaimAdmin">提交</button>
        </div>
      </div>
    </div>
  </LtPageShell>

  <!-- 移动端：原有布局 -->
  <div v-else class="am-settings">
    <!-- Page Header -->
    <div class="am-page-header mb-5">
      <h1 class="text-h5 font-weight-bold">设置</h1>
      <p class="text-body-2 text-medium-emphasis mt-1">管理你的账户和偏好</p>
    </div>

    <!-- User Info -->
    <v-card class="mb-5 am-card" elevation="0">
      <v-card-text class="pa-5">
        <div class="d-flex align-center">
          <div class="am-user-avatar">
            <v-icon color="primary" size="28">mdi-account</v-icon>
          </div>
          <div class="ml-4">
            <div class="text-h6 font-weight-bold">{{ store.deviceUser?.display_name || '未登录' }}</div>
            <div class="text-caption text-medium-emphasis mt-1">
              <v-chip
                size="x-small"
                :color="store.deviceUser?.role === 'admin' ? 'primary' : 'default'"
                variant="tonal"
                rounded="lg"
              >
                {{ store.deviceUser?.role === 'admin' ? '管理员' : '用户' }}
              </v-chip>
            </div>
          </div>
        </div>
        <v-btn
          variant="outlined"
          color="primary"
          prepend-icon="mdi-pencil-outline"
          block
          rounded="xl"
          class="mt-4"
          @click="openNameEditor"
        >
          修改名称
        </v-btn>
      </v-card-text>
    </v-card>

    <!-- Appearance -->
    <v-card class="mb-5 am-card" elevation="0">
      <v-card-title class="text-subtitle-1 font-weight-bold pa-4 pb-2">
        外观
      </v-card-title>
      <v-card-text class="pa-4 pt-0">
        <v-list-item
          title="主题"
          :subtitle="currentThemeLabel"
          @click="cycleTheme"
          clickable
          class="am-setting-item"
        >
          <template #prepend>
            <div class="am-setting-icon">
              <v-icon :icon="currentThemeIcon" color="primary" size="22" />
            </div>
          </template>
          <template #append>
            <v-icon>mdi-chevron-right</v-icon>
          </template>
        </v-list-item>
      </v-card-text>
    </v-card>

    <!-- Admin -->
    <v-card v-if="store.deviceUser?.role !== 'admin'" class="mb-5 am-card" elevation="0">
      <v-card-title class="text-subtitle-1 font-weight-bold pa-4 pb-2">
        管理员
      </v-card-title>
      <v-card-text class="pa-4 pt-0">
        <p class="text-body-2 text-medium-emphasis mb-4">
          输入 config.toml 中设置的管理员令牌以获取管理员权限。
        </p>
        <v-btn
          color="primary"
          prepend-icon="mdi-shield-key-outline"
          block
          rounded="xl"
          @click="showAdminClaim = true"
        >
          申请管理员权限
        </v-btn>
      </v-card-text>
    </v-card>

    <v-card v-else class="mb-5 am-card" elevation="0">
      <v-card-title class="text-subtitle-1 font-weight-bold pa-4 pb-2">
        管理员
      </v-card-title>
      <v-card-text class="pa-4 pt-0">
        <v-btn
          color="primary"
          prepend-icon="mdi-shield-account-outline"
          block
          rounded="xl"
          @click="router.push('/admin')"
        >
          进入管理后台
        </v-btn>
      </v-card-text>
    </v-card>

    <!-- Name Editor Dialog -->
    <v-dialog v-model="showNameEditor" max-width="400">
      <v-card rounded="xl">
        <v-card-title class="pa-5 pb-3">设置显示名称</v-card-title>
        <v-card-text class="px-5 pb-2">
          <v-text-field
            v-model="editName"
            placeholder="输入名称"
            maxlength="32"
            autofocus
            @keyup.enter="saveName"
          />
        </v-card-text>
        <v-card-actions class="pa-5 pt-2">
          <v-spacer />
          <v-btn variant="text" @click="showNameEditor = false">取消</v-btn>
          <v-btn color="primary" rounded="xl" @click="saveName">保存</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Admin Claim Dialog -->
    <v-dialog v-model="showAdminClaim" max-width="400">
      <v-card rounded="xl">
        <v-card-title class="pa-5 pb-3">申请管理员权限</v-card-title>
        <v-card-text class="px-5 pb-2">
          <p class="text-body-2 text-medium-emphasis mb-4">
            输入 config.toml 中设置的管理员令牌
          </p>
          <v-text-field
            v-model="adminToken"
            placeholder="管理员设置令牌"
            type="password"
            autofocus
            @keyup.enter="doClaimAdmin"
          />
        </v-card-text>
        <v-card-actions class="pa-5 pt-2">
          <v-spacer />
          <v-btn variant="text" @click="showAdminClaim = false">取消</v-btn>
          <v-btn color="primary" rounded="xl" @click="doClaimAdmin">提交</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
/* ─── LT 桌面端样式 ─── */
.lt-card {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-md);
  padding: 16px;
  box-shadow: var(--lt-shadow-subtle);
}

.lt-card-title {
  font-family: var(--lt-font-serif);
  font-size: 16px;
  font-weight: 700;
  color: var(--lt-text-primary);
  margin-bottom: 12px;
}

.lt-user-card {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.lt-user-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.lt-user-avatar-lg {
  width: 56px;
  height: 56px;
  border-radius: var(--lt-radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.lt-user-meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
  /* 规则3: 弹性收缩 + 防溢出 */
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.lt-user-name-lg {
  font-family: var(--lt-font-serif);
  /* 规则1: 响应式缩小字号 */
  font-size: clamp(14px, 3vw, 20px);
  font-weight: 700;
  color: var(--lt-text-primary);
  /* 规则2: 允许换行 */
  overflow-wrap: break-word;
  word-break: break-word;
  overflow: hidden;
  max-width: 100%;
}

.lt-chip {
  display: inline-block;
  font-size: 10px;
  font-weight: 600;
  color: var(--lt-text-secondary);
  background: var(--lt-btn-bg);
  padding: 2px 8px;
  border-radius: 10px;
  width: fit-content;
}

.lt-chip.admin { background: var(--lt-accent); color: #fff; }

.lt-btn-outline {
  width: 100%;
  padding: 10px 14px;
  border: 1px solid var(--lt-divider);
  border-radius: var(--lt-radius-sm);
  background: transparent;
  color: var(--lt-text-primary);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
  text-align: center;
  font-family: var(--lt-font-sans);
}
.lt-btn-outline:hover { background: var(--lt-btn-bg); }

.lt-btn-primary {
  padding: 10px 20px;
  border: none;
  border-radius: var(--lt-radius-sm);
  background: var(--lt-play-btn-bg);
  color: var(--lt-play-btn-icon);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
  font-family: var(--lt-font-sans);
}
.lt-btn-primary:hover { opacity: 0.88; }
.lt-btn-primary.sm { padding: 6px 14px; font-size: 12px; }

.lt-btn-text {
  padding: 6px 14px;
  border: none;
  background: transparent;
  color: var(--lt-text-secondary);
  font-size: 13px;
  cursor: pointer;
  font-family: var(--lt-font-sans);
}
.lt-btn-text:hover { color: var(--lt-text-primary); }

.lt-setting-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 8px;
  border-radius: var(--lt-radius-sm);
  cursor: pointer;
  transition: background 0.15s;
}
.lt-setting-row:hover { background: var(--lt-btn-bg); }

.lt-setting-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--lt-btn-bg);
  border-radius: var(--lt-radius-sm);
  color: var(--lt-text-secondary);
  flex-shrink: 0;
}

.lt-setting-info { flex: 1; }
.lt-setting-label { font-size: 14px; font-weight: 600; color: var(--lt-text-primary); }
.lt-setting-desc { font-size: 12px; color: var(--lt-text-secondary); margin-top: 2px; }

.lt-chevron { width: 18px; height: 18px; color: var(--lt-text-muted); flex-shrink: 0; }

.lt-info-text {
  font-size: 13px;
  line-height: 1.5;
  color: var(--lt-text-secondary);
  margin-bottom: 12px;
}
.lt-info-text.mb { margin-bottom: 16px; }

.lt-input {
  width: 100%;
  border: 1px solid var(--lt-divider);
  outline: none;
  background: transparent;
  font-size: 14px;
  color: var(--lt-text-primary);
  font-family: var(--lt-font-sans);
  padding: 10px 12px;
  border-radius: var(--lt-radius-sm);
}
.lt-input:focus { border-color: var(--lt-text-secondary); }
.lt-input::placeholder { color: var(--lt-text-muted); }

.lt-modal-title {
  font-family: var(--lt-font-serif);
  font-size: 18px;
  font-weight: 700;
  color: var(--lt-text-primary);
  flex: 1;
}

.lt-search-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 300;
}

.lt-search-modal {
  background: var(--lt-card-bg);
  border-radius: var(--lt-radius-lg);
  width: 90%;
  max-width: 460px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: var(--lt-shadow-card);
}
.lt-search-modal.sm-modal { max-width: 360px; }

.lt-search-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--lt-divider);
}

.lt-search-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--lt-text-secondary);
  cursor: pointer;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}
.lt-search-close:hover { background: var(--lt-btn-bg); }
.lt-search-close svg { width: 18px; height: 18px; }

.lt-modal-body { padding: 16px; }
.lt-modal-actions { display: flex; justify-content: flex-end; gap: 8px; padding: 0 16px 14px; }

/* ─── 移动端原有样式 ─── */
.am-settings {
  padding-bottom: 16px;
  animation: slideUp 0.5s var(--am-ease-emphasized);
}

.am-page-header {
  animation: slideUp 0.4s var(--am-ease-emphasized);
}

.am-card {
  overflow: hidden;
}

.am-card:hover {
  border-color: transparent;
}

.am-user-avatar {
  width: 56px;
  height: 56px;
  border-radius: var(--am-radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--am-surface-2);
  border: 1px solid var(--am-divider);
}

.am-setting-item {
  border-radius: var(--am-radius-sm);
  transition: background-color 0.2s var(--am-ease-emphasized);
}

.am-setting-item:hover {
  background: var(--am-surface-2);
}

.am-setting-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--am-surface-2);
  border-radius: var(--am-radius-sm);
  margin-right: 12px;
}

@keyframes slideUp {
  from { transform: translateY(16px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}
</style>
