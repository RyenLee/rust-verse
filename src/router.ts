import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'dashboard',
      component: () => import('@/views/DashboardView.vue'),
    },
    {
      path: '/toolchains',
      name: 'toolchains',
      component: () => import('@/views/ToolchainListView.vue'),
    },
    {
      path: '/history-versions',
      name: 'history-versions',
      component: () => import('@/views/HistoryVersionView.vue'),
    },
    {
      path: '/components',
      name: 'components',
      component: () => import('@/views/ComponentsView.vue'),
    },
    {
      path: '/targets',
      name: 'targets',
      component: () => import('@/views/TargetsView.vue'),
    },
    {
      path: '/overrides',
      name: 'overrides',
      component: () => import('@/views/OverrideView.vue'),
    },
    {
      path: '/updates',
      name: 'updates',
      component: () => import('@/views/UpdateView.vue'),
    },
    {
      path: '/plugins',
      name: 'plugins',
      component: () => import('@/views/PluginsView.vue'),
    },
    {
      path: '/env-vars',
      name: 'env-vars',
      component: () => import('@/views/EnvVarsView.vue'),
    },
    {
      path: '/mirrors',
      name: 'mirrors',
      component: () => import('@/views/MirrorView.vue'),
    },
    {
      path: '/about',
      name: 'about',
      component: () => import('@/views/AppUpdateView.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/SettingsView.vue'),
    },
    {
      path: '/notifications',
      name: 'notifications',
      component: () => import('@/views/NotificationCenter.vue'),
    },
  ],
})

export default router
