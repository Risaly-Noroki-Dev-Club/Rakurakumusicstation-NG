import { createApp } from 'vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import 'vuetify/styles'

import App from './App.vue'
import router from './router'
import './style.css'

// Lucide icons for Vuetify custom icon set
import {
  CirclePlay, CirclePause, SkipForward, SkipBack, Repeat, X, Plus, Search,
  Radio, BookOpen, ListOrdered, Settings, ShieldCheck, Shield, User,
  Users, Music, Music2, ListMusic, Download, Upload, CloudUpload,
  Image, RefreshCw, Trash2, ChevronRight, ChevronDown, ChevronLeft, FileMusic,
  FileText, Volume2, History, Pencil, Info, Save, Cable, BarChart3,
  CirclePlus, ListPlus, ListX, Cloud, Sun, Moon, Monitor
} from '@lucide/vue'

const lucideIcons: Record<string, any> = {
  'mdi-play-circle-outline': CirclePlay,
  'mdi-bookshelf': BookOpen,
  'mdi-format-list-numbered': ListOrdered,
  'mdi-cog-outline': Settings,
  'mdi-radio-tower': Radio,
  'mdi-shield-account-outline': ShieldCheck,
  'mdi-account-circle-outline': User,
  'mdi-play-circle': CirclePlay,
  'mdi-music-note': Music,
  'mdi-account-group': Users,
  'mdi-skip-previous': SkipBack,
  'mdi-pause': CirclePause,
  'mdi-play': CirclePlay,
  'mdi-skip-next': SkipForward,
  'mdi-text-box-outline': FileText,
  'mdi-magnify': Search,
  'mdi-plus-circle-outline': CirclePlus,
  'mdi-volume-high': Volume2,
  'mdi-music-note-outline': Music2,
  'mdi-replay': Repeat,
  'mdi-close': X,
  'mdi-history': History,
  'mdi-playlist-plus': ListPlus,
  'mdi-download-outline': Download,
  'mdi-cloud-upload-outline': CloudUpload,
  'mdi-playlist-music-outline': ListMusic,
  'mdi-delete-outline': Trash2,
  'mdi-information-outline': Info,
  'mdi-playlist-remove': ListX,
  'mdi-theme-light-dark': Monitor,
  'mdi-weather-night': Moon,
  'mdi-white-balance-sunny': Sun,
  'mdi-account': User,
  'mdi-pencil-outline': Pencil,
  'mdi-chevron-right': ChevronRight,
  'mdi-shield-key-outline': Shield,
  'mdi-account-group-outline': Users,
  'mdi-music-circle-outline': Music,
  'mdi-chart-bar': BarChart3,
  'mdi-refresh': RefreshCw,
  'mdi-delete': Trash2,
  'mdi-image': Image,
  'mdi-cloud-upload': CloudUpload,
  'mdi-download': Download,
  'mdi-playlist-music': ListMusic,
  'mdi-folder-music': FileMusic,
  'mdi-content-save': Save,
  'mdi-connection': Cable,
}

const vuetify = createVuetify({
  components,
  directives,
  icons: {
    defaultSet: 'custom',
    aliases: {
      ...Object.fromEntries(
        Object.entries(lucideIcons).map(([name, component]) => [name, component])
      ),
    },
    sets: {
      custom: {
        component: (props: any) => {
          const icon = lucideIcons[props.name]
          if (!icon) return null
          return icon
        },
      },
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

// Request notification permission on user interaction
let notificationPermissionRequested = false
function requestNotificationPermission() {
  if (notificationPermissionRequested) return
  if ('Notification' in window && Notification.permission === 'default') {
    notificationPermissionRequested = true
    Notification.requestPermission()
  }
}

// Listen for user interaction to request permission
document.addEventListener('click', requestNotificationPermission, { once: true })
document.addEventListener('keydown', requestNotificationPermission, { once: true })

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
