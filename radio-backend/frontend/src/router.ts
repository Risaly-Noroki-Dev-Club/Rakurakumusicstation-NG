import { createRouter, createWebHistory } from 'vue-router'
import { store } from './store'
import PlayerView from './views/PlayerView.vue'
import QueueView from './views/QueueView.vue'
import LibraryView from './views/LibraryView.vue'
import AdminView from './views/AdminView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/player' },
    { path: '/player', name: 'player', component: PlayerView },
    { path: '/queue', name: 'queue', component: QueueView },
    { path: '/library', name: 'library', component: LibraryView },
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
      return { name: 'player' }
    }
  }
})

export default router
