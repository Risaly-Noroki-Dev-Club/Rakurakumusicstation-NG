<script setup lang="ts">
defineProps<{
  message: string
  type?: 'info' | 'success' | 'error'
}>()
</script>

<template>
  <div v-if="message" class="status-msg" :class="'status-' + (type || 'info')">
    {{ message }}
  </div>
</template>

<style scoped>
.status-msg {
  padding: 10px 14px;
  border-radius: 6px;
  margin: 8px 0;
  font-size: 0.9rem;
}
.status-info {
  background: rgba(23,162,184,0.15);
  color: var(--info, #17a2b8);
}
.status-success {
  background: rgba(40,167,69,0.15);
  color: var(--success, #28a745);
}
.status-error {
  background: rgba(220,53,69,0.15);
  color: var(--danger, #dc3545);
}
</style>
