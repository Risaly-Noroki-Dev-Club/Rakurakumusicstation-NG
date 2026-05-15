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
          background: '#FAFAFA',
          surface: '#FFFFFF',
          'surface-variant': '#F5F5F5',
          primary: '#003D99',
          secondary: '#00897B',
          accent: '#FF6F00',
          error: '#B00020',
          info: '#1976D2',
          success: '#388E3C',
          warning: '#FBC02D',
          'on-background': 'rgba(0,0,0,0.87)',
          'on-surface': 'rgba(0,0,0,0.87)',
        },
      },
      dark: {
        dark: true,
        colors: {
          background: '#101114',
          surface: '#191B20',
          'surface-variant': '#23262D',
          primary: '#8AB4F8',
          secondary: '#5FD0C3',
          accent: '#FFB86B',
          error: '#FFB4AB',
          info: '#A8C7FA',
          success: '#9BD59B',
          warning: '#FDD663',
          'on-background': 'rgba(255,255,255,0.92)',
          'on-surface': 'rgba(255,255,255,0.92)',
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
      elevation: 1,
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
      rounded: 'md',
    },
    VSnackbar: {
      rounded: 'lg',
      location: 'top',
      timeout: 3000,
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
