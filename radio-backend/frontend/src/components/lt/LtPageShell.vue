<script setup lang="ts">
// 通用页面外壳：为非 Now Playing 的桌面页面提供 LT 风格的标题 + 内容卡片容器。
defineProps<{
  title: string
  subtitle?: string
}>()
</script>

<template>
  <div class="lt-page">
    <div class="lt-page-header">
      <h1 class="lt-page-title">{{ title }}</h1>
      <p v-if="subtitle" class="lt-page-subtitle">{{ subtitle }}</p>
    </div>
    <div class="lt-page-body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.lt-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  animation: lt-page-enter 0.4s ease;
}

.lt-page-header {
  flex-shrink: 0;
  padding: 0 4px 16px;
}

.lt-page-title {
  font-family: var(--lt-font-serif);
  /* 规则1: 响应式缩小字号 */
  font-size: clamp(16px, 3.5vw, 24px);
  font-weight: 700;
  color: var(--lt-text-primary);
  letter-spacing: -0.3px;
  /* 规则2: 允许换行 */
  overflow-wrap: break-word;
  word-break: break-word;
  overflow: hidden;
  max-width: 100%;
}

.lt-page-subtitle {
  font-size: 13px;
  font-weight: 400;
  color: var(--lt-text-secondary);
  margin-top: 4px;
}

.lt-page-body {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-bottom: 16px;
}

@keyframes lt-page-enter {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
