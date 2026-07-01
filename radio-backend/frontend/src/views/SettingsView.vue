<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { store, cycleTheme, THEMES, toast } from '../store'
import { setDisplayName, claimAdmin } from '../api'

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
</script>

<template>
  <div class="am-settings">
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
