<script setup lang="ts">
import { ref } from 'vue'
import { store } from '../store'
import { doAuth, closeAuth, toggleAuthMode } from '../api'

const usernameRef = ref<HTMLInputElement | null>(null)
const passwordRef = ref<HTMLInputElement | null>(null)

function handleAuth() {
  const u = usernameRef.value ? usernameRef.value.value : ''
  const p = passwordRef.value ? passwordRef.value.value : ''
  doAuth(u, p)
}
</script>

<template>
  <div v-if="store.showAuth" class="auth-overlay" @click.self="closeAuth">
    <div class="card" style="width:360px;max-width:90vw">
      <h2>{{ store.authMode === 'login' ? '登录' : '注册' }}</h2>
      <div class="form-group">
        <label>用户名</label>
        <input type="text" ref="usernameRef" autocomplete="username" placeholder="3-32个字符"
               @keyup.enter="handleAuth">
      </div>
      <div class="form-group">
        <label>密码</label>
        <input type="password" ref="passwordRef" autocomplete="current-password" placeholder="至少6个字符"
               @keyup.enter="handleAuth">
      </div>
      <div style="display:flex;gap:8px;margin-top:12px">
        <button class="btn btn-primary" @click="handleAuth">
          {{ store.authMode === 'login' ? '登录' : '注册' }}
        </button>
        <button class="btn btn-secondary" @click="toggleAuthMode">
          {{ store.authMode === 'login' ? '切换到注册' : '切换到登录' }}
        </button>
        <button class="btn btn-secondary" @click="closeAuth">取消</button>
      </div>
      <div v-if="store.authError" style="color:var(--danger);margin-top:8px;font-size:0.85em">
        {{ store.authError }}
      </div>
    </div>
  </div>
</template>
