// Keeps Vuetify's theme in sync with the app theme setting and system theme.

import { onUnmounted, watch } from 'vue'
import type { ThemeInstance } from 'vuetify'
import { store, THEMES } from '../store'

export function useThemeSync(vuetifyTheme: ThemeInstance) {
  function preferredTheme(): 'light' | 'dark' {
    const selected = THEMES[store.themeIdx]
    if (selected === 'dark' || selected === 'light') return selected
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  function syncTheme() {
    vuetifyTheme.global.name.value = preferredTheme()
  }

  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  const onSystemThemeChange = () => {
    if (THEMES[store.themeIdx] === 'auto') syncTheme()
  }

  watch(() => store.themeIdx, syncTheme, { immediate: true })
  mediaQuery.addEventListener('change', onSystemThemeChange)

  onUnmounted(() => {
    mediaQuery.removeEventListener('change', onSystemThemeChange)
  })
}
