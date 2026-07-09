// 桌面端全局布局切换逻辑。
// 在 store.isDesktop 时为所有路由激活 Listen Together 主题（data-layout 属性），
// 使 --lt-* 暖色变量在所有页面生效。移动端不激活。

import { computed, watch } from 'vue'
import { store } from '../store'

export function useListenTogetherLayout() {
  const isActive = computed(() => store.isDesktop)

  watch(
    isActive,
    (active) => {
      const root = document.documentElement
      if (active) {
        root.setAttribute('data-layout', 'listen-together')
      } else {
        root.removeAttribute('data-layout')
      }
    },
    { immediate: true }
  )

  return { isActive }
}
