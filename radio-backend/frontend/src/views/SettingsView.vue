<script setup lang="ts">
import { ref } from 'vue'
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
  auto: '自动',
  light: '浅色',
  dark: '深色',
}

const currentThemeLabel = computed(() => {
  return themeLabels[THEMES[store.themeIdx]] || '自动'
})

import { computed } from 'vue'
</script>

<template>
  <div class="am-settings">
    <!-- User Info -->
    <v-card class="mb-4" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold">
        用户信息
      </v-card-title>
      <v-card-text>
        <div class="d-flex align-center mb-4">
          <v-avatar color="primary" size="48" class="mr-4">
            <v-icon color="white" size="28">mdi-account</v-icon>
          </v-avatar>
          <div>
            <div class="text-h6 font-weight-bold">{{ store.deviceUser?.display_name || '未登录' }}</div>
            <div class="text-caption text-medium-emphasis">
              <v-chip
                size="x-small"
                :color="store.deviceUser?.role === 'admin' ? 'primary' : 'default'"
                variant="tonal"
              >
                {{ store.deviceUser?.role === 'admin' ? '管理员' : '用户' }}
              </v-chip>
            </div>
          </div>
        </div>

        <v-btn
          variant="outlined"
          color="primary"
          prepend-icon="mdi-pencil"
          block
          @click="openNameEditor"
        >
          修改名称
        </v-btn>
      </v-card-text>
    </v-card>

    <!-- Appearance -->
    <v-card class="mb-4" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold">
        外观
      </v-card-title>
      <v-card-text>
        <v-list-item
          title="主题"
          :subtitle="currentThemeLabel"
          @click="cycleTheme"
          clickable
        >
          <template #prepend>
            <v-icon color="primary">mdi-theme-light-dark</v-icon>
          </template>
          <template #append>
            <v-icon>mdi-chevron-right</v-icon>
          </template>
        </v-list-item>
      </v-card-text>
    </v-card>

    <!-- Admin -->
    <v-card v-if="store.deviceUser?.role !== 'admin'" class="mb-4" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold">
        管理员
      </v-card-title>
      <v-card-text>
        <p class="text-body-2 text-medium-emphasis mb-4">
          输入 config.toml 中设置的管理员令牌以获取管理员权限。
        </p>
        <v-btn
          color="primary"
          prepend-icon="mdi-shield-key"
          block
          @click="showAdminClaim = true"
        >
          申请管理员权限
        </v-btn>
      </v-card-text>
    </v-card>

    <v-card v-else class="mb-4" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold">
        管理员
      </v-card-title>
      <v-card-text>
        <v-btn
          color="primary"
          prepend-icon="mdi-shield-account"
          block
          @click="router.push('/admin')"
        >
          进入管理后台
        </v-btn>
      </v-card-text>
    </v-card>

    <!-- Name Editor Dialog -->
    <v-dialog v-model="showNameEditor" max-width="400">
      <v-card>
        <v-card-title>设置显示名称</v-card-title>
        <v-card-text>
          <v-text-field
            v-model="editName"
            placeholder="输入名称"
            maxlength="32"
            autofocus
            @keyup.enter="saveName"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="showNameEditor = false">取消</v-btn>
          <v-btn color="primary" @click="saveName">保存</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Admin Claim Dialog -->
    <v-dialog v-model="showAdminClaim" max-width="400">
      <v-card>
        <v-card-title>申请管理员权限</v-card-title>
        <v-card-text>
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
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="showAdminClaim = false">取消</v-btn>
          <v-btn color="primary" @click="doClaimAdmin">提交</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.am-settings {
  padding-bottom: 16px;
}
</style>
