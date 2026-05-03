<script setup lang="ts">
import { store } from '../store'
</script>

<template>
  <div class="toast-container">
    <div v-for="t in store.toasts" :key="t.id" :class="['toast', 'toast-' + t.level]">
      {{ t.message }}
    </div>
  </div>
</template>
