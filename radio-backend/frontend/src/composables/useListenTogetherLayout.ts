// 桌面端 Now Playing 路由的三栏布局切换逻辑。
// 仅在 store.isDesktop 且处于 now-playing 路由时激活，并在 <html> 上设置
// data-layout="listen-together"，使 --lt-* 主题变量生效。

import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { store } from '../store'

export function useListenTogetherLayout() {
  const route = useRoute()
  const isActive = computed(
    () => store.isDesktop && route.name === 'now-playing'
  )

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
