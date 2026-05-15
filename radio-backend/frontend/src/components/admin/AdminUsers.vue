<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { store, toast } from '../../store'
import { apiFetch } from '../../api'

const loading = ref(false)

async function loadUsers() {
  loading.value = true
  try {
    const r = await apiFetch('/api/admin/users')
    const d = await r.json()
    if (d.success) store.users = d.data || []
  } catch { /* ignore */ }
  try {
    const r = await apiFetch('/api/admin/logs')
    const d = await r.json()
    if (d.success) store.adminLogs = d.data || []
  } catch { /* ignore */ }
  loading.value = false
}

async function adminPromote(id: number) {
  try {
    const r = await apiFetch('/api/admin/users/' + id + '/role', {
      method: 'PUT', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'admin' })
    })
    const data = await r.json()
    if (data.success) { loadUsers(); toast('用户已提升为管理员', 'success') }
    else toast('操作失败: ' + data.error, 'error')
  } catch { toast('请求失败', 'error') }
}

async function adminDemote(id: number) {
  try {
    const r = await apiFetch('/api/admin/users/' + id + '/role', {
      method: 'PUT', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'user' })
    })
    const data = await r.json()
    if (data.success) { loadUsers(); toast('用户已降级为用户', 'success') }
    else toast('操作失败: ' + data.error, 'error')
  } catch { toast('请求失败', 'error') }
}

async function adminBan(id: number) {
  try {
    const r = await apiFetch('/api/admin/users/' + id + '/ban', { method: 'POST' })
    const d = await r.json()
    if (d.success) { loadUsers(); toast('用户已封禁', 'success') }
    else toast('操作失败: ' + d.error, 'error')
  } catch { toast('请求失败', 'error') }
}

async function adminUnban(id: number) {
  try {
    const r = await apiFetch('/api/admin/users/' + id + '/unban', { method: 'POST' })
    const d = await r.json()
    if (d.success) { loadUsers(); toast('用户已解封', 'success') }
    else toast('操作失败: ' + d.error, 'error')
  } catch { toast('请求失败', 'error') }
}

onMounted(() => {
  loadUsers()
})

const headers = [
  { title: 'ID', key: 'id', width: '60' },
  { title: '用户名', key: 'display_name' },
  { title: '角色', key: 'role' },
  { title: '状态', key: 'status' },
  { title: '操作', key: 'actions', sortable: false },
]
</script>

<template>
  <div>
    <v-data-table
      :headers="headers"
      :items="store.users"
      density="compact"
      class="elevation-0"
      :loading="loading"
      :items-per-page="20"
      :items-per-page-options="[10, 20, 50, 100]"
    >
      <template #item.role="{ item }">
        <v-chip
          size="small"
          :color="item.role === 'admin' ? 'primary' : 'default'"
          variant="tonal"
        >
          {{ item.role === 'admin' ? '管理员' : '用户' }}
        </v-chip>
      </template>

      <template #item.status="{ item }">
        <v-chip
          size="small"
          :color="(item as any).is_banned ? 'error' : 'success'"
          variant="tonal"
        >
          {{ (item as any).is_banned ? '已封禁' : '正常' }}
        </v-chip>
      </template>

      <template #item.actions="{ item }">
        <div class="d-flex gap-1">
          <v-btn
            v-if="item.role !== 'admin'"
            size="x-small"
            color="primary"
            variant="text"
            @click="adminPromote(item.id)"
          >
            提权
          </v-btn>
          <v-btn
            v-if="item.role === 'admin'"
            size="x-small"
            color="warning"
            variant="text"
            @click="adminDemote(item.id)"
          >
            降权
          </v-btn>
          <v-btn
            v-if="(item as any).is_banned"
            size="x-small"
            color="success"
            variant="text"
            @click="adminUnban(item.id)"
          >
            解封
          </v-btn>
          <v-btn
            v-else
            size="x-small"
            color="error"
            variant="text"
            @click="adminBan(item.id)"
          >
            封禁
          </v-btn>
        </div>
      </template>
    </v-data-table>

    <v-card class="mt-4" elevation="1">
      <v-card-title class="text-subtitle-1 font-weight-bold">
        操作日志（最近100条）
      </v-card-title>
      <v-card-text>
        <div class="max-h-300 overflow-y-auto">
          <div
            v-for="(l, idx) in store.adminLogs"
            :key="idx"
            class="py-1 text-caption"
            :class="{ 'border-b': idx < store.adminLogs.length - 1 }"
          >
            <span class="text-medium-emphasis">[{{ l.created_at }}]</span>
            {{ l.action }} — {{ l.details }}
          </div>
        </div>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>
.gap-1 {
  gap: 4px;
}
.max-h-300 {
  max-height: 300px;
}
.border-b {
  border-bottom: 1px solid var(--am-divider);
}
</style>
