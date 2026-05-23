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
      path: '/help',
      name: 'help',
      component: () => import('@/views/HelpView.vue'),
    },
  ],
})

export default router
