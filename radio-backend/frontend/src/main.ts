import { createApp } from 'vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import { aliases, mdi } from 'vuetify/iconsets/mdi'
import '@mdi/font/css/materialdesignicons.css'
import 'vuetify/styles'

import App from './App.vue'
import router from './router'
import './style.css'

const vuetify = createVuetify({
  components,
  directives,
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: {
      mdi,
    },
  },
  theme: {
    defaultTheme: 'light',
    themes: {
      light: {
        dark: false,
        colors: {
          background: '#F7F7F8',
          surface: '#FFFFFF',
          'surface-variant': '#F0F0F2',
          primary: '#6C5CE7',
          secondary: '#00CEC9',
          accent: '#FD79A8',
          error: '#E17055',
          info: '#74B9FF',
          success: '#00B894',
          warning: '#FDCB6E',
          'on-background': 'rgba(28, 25, 52, 0.95)',
          'on-surface': 'rgba(28, 25, 52, 0.95)',
        },
      },
      dark: {
        dark: true,
        colors: {
          background: '#0C0B10',
          surface: '#16141C',
          'surface-variant': '#1C1A24',
          primary: '#A29BFE',
          secondary: '#55EFC4',
          accent: '#FD79A8',
          error: '#E17055',
          info: '#74B9FF',
          success: '#00B894',
          warning: '#FDCB6E',
          'on-background': 'rgba(240, 236, 255, 0.94)',
          'on-surface': 'rgba(240, 236, 255, 0.94)',
        },
      },
    },
  },
  defaults: {
    global: {
      ripple: true,
    },
    VBtn: {
      elevation: 0,
      rounded: 'lg',
    },
    VCard: {
      elevation: 0,
      rounded: 'lg',
    },
    VTextField: {
      variant: 'outlined',
      density: 'comfortable',
      hideDetails: 'auto',
    },
    VTextarea: {
      variant: 'outlined',
      density: 'comfortable',
      hideDetails: 'auto',
    },
    VSelect: {
      variant: 'outlined',
      density: 'comfortable',
      hideDetails: 'auto',
    },
    VListItem: {
      rounded: 'lg',
    },
    VSnackbar: {
      rounded: 'lg',
      location: 'top',
      timeout: 3000,
    },
    VDialog: {
      maxWidth: 560,
    },
  },
})

const app = createApp(App)
app.use(vuetify)
app.use(router)
app.mount('#app')

// Register Service Worker for PWA
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    const base = import.meta.env.BASE_URL
    navigator.serviceWorker.register(base + 'sw.js', { scope: base })
      .then(reg => {
        console.log('[PWA] Service Worker registered:', reg.scope)
      })
      .catch(err => {
        console.warn('[PWA] Service Worker registration failed:', err)
      })
  })
}
