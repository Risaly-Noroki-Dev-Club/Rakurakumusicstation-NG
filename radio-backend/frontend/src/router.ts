import { createRouter, createWebHistory } from 'vue-router'
import { store } from './store'
import NowPlayingView from './views/NowPlayingView.vue'
import LibraryView from './views/LibraryView.vue'
import UpNextView from './views/UpNextView.vue'
import SettingsView from './views/SettingsView.vue'
import AdminView from './views/AdminView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/', name: 'now-playing', component: NowPlayingView },
    { path: '/library', name: 'library', component: LibraryView },
    { path: '/queue', redirect: '/up-next' },
    { path: '/up-next', name: 'up-next', component: UpNextView },
    { path: '/settings', name: 'settings', component: SettingsView },
    { path: '/admin', redirect: '/admin/users' },
    {
      path: '/admin/:subtab',
      name: 'admin',
      component: AdminView,
      props: true,
      meta: { requiresAdmin: true }
    },
  ]
})

router.beforeEach((to) => {
  if (to.meta.requiresAdmin) {
    const user = store.deviceUser
    if (!user || user.role !== 'admin') {
      return { name: 'now-playing' }
    }
  }
})

export default router
