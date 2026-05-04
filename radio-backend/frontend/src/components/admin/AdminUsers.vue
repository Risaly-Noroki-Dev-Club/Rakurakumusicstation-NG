<script setup lang="ts">
import { store, toast } from '../../store'
import { getBackendUrl } from '../../api'

const ah = (): Record<string, string> => {
  const h: Record<string, string> = {}
  if (store.token) h['Authorization'] = 'Bearer ' + store.token
  return h
}

async function loadUsers() {
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/users', { headers: ah() })
    const d = await r.json()
    if (d.success) store.users = d.data || []
  } catch { /* ignore */ }
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/logs', { headers: ah() })
    const d = await r.json()
    if (d.success) store.adminLogs = d.data || []
  } catch { /* ignore */ }
}

async function adminPromote(id: number) {
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/users/' + id + '/role', {
      method: 'PUT', headers: { ...ah(), 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'admin' })
    })
    const data = await r.json()
    if (data.success) { loadUsers(); toast('用户已提升为管理员', 'success') }
    else toast('操作失败: ' + data.error, 'error')
  } catch { toast('请求失败', 'error') }
}

async function adminDemote(id: number) {
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/users/' + id + '/role', {
      method: 'PUT', headers: { ...ah(), 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'user' })
    })
    const data = await r.json()
    if (data.success) { loadUsers(); toast('用户已降级为用户', 'success') }
    else toast('操作失败: ' + data.error, 'error')
  } catch { toast('请求失败', 'error') }
}

async function adminBan(id: number) {
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/users/' + id + '/ban', { method: 'POST', headers: ah() })
    const d = await r.json()
    if (d.success) { loadUsers(); toast('用户已封禁', 'success') }
    else toast('操作失败: ' + d.error, 'error')
  } catch { toast('请求失败', 'error') }
}

async function adminUnban(id: number) {
  try {
    const r = await fetch(getBackendUrl() + '/api/admin/users/' + id + '/unban', { method: 'POST', headers: ah() })
    const d = await r.json()
    if (d.success) { loadUsers(); toast('用户已解封', 'success') }
    else toast('操作失败: ' + d.error, 'error')
  } catch { toast('请求失败', 'error') }
}

const emit = defineEmits<{ load: [] }>()
loadUsers()
</script>

<template>
  <div>
    <div class="card admin-panel">
      <h2>👥 用户管理</h2>
      <table class="users-table">
        <thead><tr><th>ID</th><th>用户名</th><th>角色</th><th>状态</th><th>操作</th></tr></thead>
        <tbody>
          <tr v-for="u in store.users" :key="u.id">
            <td>{{ u.id }}</td><td>{{ u.username }}</td>
            <td>
              <span :class="['role-badge', u.role === 'admin' ? 'role-admin' : 'role-user']">
                {{ u.role === 'admin' ? '管理员' : '用户' }}
              </span>
            </td>
            <td>{{ (u as any).is_banned ? '🔴已封禁' : '🟢正常' }}</td>
            <td>
              <button v-if="u.role !== 'admin'" class="btn btn-primary btn-small" @click="adminPromote(u.id)">⭐ 提权</button>
              <button v-if="u.role === 'admin'" class="btn btn-warning btn-small" @click="adminDemote(u.id)">⬇ 降权</button>
              <button v-if="(u as any).is_banned" class="btn btn-secondary btn-small" @click="adminUnban(u.id)">解封</button>
              <button v-else class="btn btn-danger btn-small" @click="adminBan(u.id)">封禁</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div class="card admin-panel">
      <h2>📋 操作日志 (最近100条)</h2>
      <div style="max-height:300px;overflow-y:auto;font-size:0.85em">
        <div v-for="(l, idx) in store.adminLogs" :key="idx" style="padding:4px 0;border-bottom:1px solid rgba(0,0,0,0.05)">
          <span style="color:var(--text-muted)">[{{ l.created_at }}]</span>
          {{ l.action }} — {{ l.details }}
        </div>
      </div>
    </div>
  </div>
</template>
