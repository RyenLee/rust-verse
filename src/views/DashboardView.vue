<script setup lang="ts">
import { onMounted, ref, onActivated, onUnmounted, inject } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import PageLayout from '../components/PageLayout.vue'
import SectionTitle from '../components/SectionTitle.vue'
import { useRustup, type EnvCheck, type VersionInfo } from '../composables/useRustup'
import { useDataRefresh } from '../composables/useDataRefresh'
import { withTimeout } from '../composables/useWithTimeout'

const { t } = useI18n()
const { checkEnv, listToolchains, checkUpdate, getVersions } = useRustup()
const { onToolchainChange, onEnvVarChange } = useDataRefresh()
const triggerUninstall = inject<() => void>('triggerUninstall', () => {})

const envCheck = ref<EnvCheck | null>(null)
const defaultToolchain = ref<string>('-')
const toolchainCount = ref(0)
const updateCount = ref(0)
const versions = ref<VersionInfo | null>(null)
const loading = ref(true)
const loaded = ref(false)
const updateError = ref(false)

async function loadData() {
  const t0 = performance.now()
  loading.value = true
  updateError.value = false

  try {
    const r = await withTimeout(checkEnv(), 15000)
    if (!r.ok) {
      // checkEnv timeout
    } else {
      envCheck.value = r.value
    }
  } catch {
    // checkEnv error
  }

  if (!envCheck.value) {
    envCheck.value = {
      rustup_installed: false,
      cargo_installed: false,
      rustup_error: null,
      cargo_error: null,
      cargo_home: null,
      rustup_home: null,
    }
    loading.value = false
    loaded.value = true
    return
  }

  if (!envCheck.value.rustup_installed) {
    loading.value = false
    loaded.value = true
    return
  }

  const [tlResult, cuResult, gvResult] = await Promise.allSettled([
    withTimeout(listToolchains(), 20000),
    withTimeout(checkUpdate(), 30000),
    withTimeout(getVersions(), 10000),
  ])

  if (tlResult.status === 'fulfilled' && tlResult.value.ok) {
    const toolchains = tlResult.value.value
    toolchainCount.value = toolchains.length
    const def = toolchains.find(t => t.is_default)
    if (def) defaultToolchain.value = def.name
  }

  if (cuResult.status === 'fulfilled' && cuResult.value.ok) {
    const updates = cuResult.value.value
    updateCount.value = updates.filter(u => !u.up_to_date).length
    updateError.value = false
  } else {
    updateError.value = true
  }

  if (gvResult.status === 'fulfilled' && gvResult.value.ok) {
    versions.value = gvResult.value.value
  }

  loading.value = false
  loaded.value = true
}

onMounted(() => {
  if (!loaded.value) {
    loadData()
  }
})

const stopToolchainWatch = onToolchainChange(() => {
  loadData()
})

const stopEnvVarWatch = onEnvVarChange(() => {
  loadData()
})

onUnmounted(() => {
  stopToolchainWatch()
  stopEnvVarWatch()
})

onActivated(() => {
  // Keep-alive: do NOT reload on re-activation
})
</script>

<template>
  <PageLayout :group="t('nav.group.overview')" :title="t('dashboard.title')" :description="t('dashboard.description')">
    <template #actions>
      <BaseButton variant="secondary" :loading="loading" @click="loadData">
        <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
        {{ t('common.action.refresh') }}
      </BaseButton>
    </template>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center h-full">
      <div class="text-center space-y-3">
        <div class="inline-flex items-center justify-center w-12 h-12 rounded-full bg-sky-50 dark:bg-sky-900/30">
          <svg class="animate-spin h-6 w-6 text-sky-600 dark:text-sky-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
        </div>
        <p class="text-gray-500 dark:text-gray-400 text-sm">{{ t('common.status.loading') }}</p>
      </div>
    </div>

    <!-- Dashboard content -->
    <div v-else class="space-y-10">
      <!-- Rustup not installed warning -->
      <div v-if="envCheck && !envCheck.rustup_installed" class="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl p-5">
        <div class="flex items-start gap-4">
          <div class="w-10 h-10 rounded-lg bg-orange-100 dark:bg-orange-900/30 flex items-center justify-center shrink-0">
            <iconify-icon icon="mdi:alert-circle-outline" width="20" class="text-orange-500"></iconify-icon>
          </div>
          <div class="flex-1 space-y-2">
            <h3 class="text-sm font-semibold text-amber-800 dark:text-amber-300">{{ t('dashboard.rustNotFound') }}</h3>
            <p class="text-sm text-amber-700 dark:text-amber-400" v-html="t('dashboard.rustNotFoundDesc', { rustup: '<strong>rustup</strong>' })" />
            <div v-if="envCheck.rustup_error" class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-3 text-left">
              <p class="text-xs font-mono text-red-600 dark:text-red-400 break-all">{{ envCheck.rustup_error }}</p>
            </div>
            <div class="flex gap-3 pt-1">
              <a href="https://rustup.rs" target="_blank" class="inline-flex items-center gap-2 bg-orange-600 hover:bg-orange-500 text-white text-sm font-semibold py-2 px-4 rounded-lg transition-colors">
                <iconify-icon icon="mdi:download" width="16"></iconify-icon>
                {{ t('dashboard.action.installRustup') }}
              </a>
              <a href="https://www.rust-lang.org/tools/install" target="_blank" class="inline-flex items-center gap-2 text-sm text-amber-600 dark:text-amber-400 hover:text-amber-800 dark:hover:text-amber-300 transition-colors py-2 px-4">
                {{ t('dashboard.action.viewGuide') }}
              </a>
            </div>
          </div>
        </div>
      </div>

      <!-- Primary stats -->
      <section>
        <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-5">
          <!-- Default Toolchain card -->
          <div class="bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-200 dark:border-gray-700 hover:shadow-sm transition-shadow">
            <div class="flex items-center gap-3 mb-3">
              <div class="w-9 h-9 rounded-lg bg-sky-50 dark:bg-sky-900/30 flex items-center justify-center shrink-0">
                <iconify-icon icon="mdi:wrench-outline" width="18" class="text-sky-600 dark:text-sky-400"></iconify-icon>
              </div>
              <span class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">{{ t('dashboard.card.defaultToolchain') }}</span>
            </div>
            <p class="text-lg font-semibold text-gray-900 dark:text-gray-100 truncate" :title="defaultToolchain">{{ defaultToolchain }}</p>
          </div>

          <!-- Installed Toolchains card -->
          <div class="bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-200 dark:border-gray-700 hover:shadow-sm transition-shadow">
            <div class="flex items-center gap-3 mb-3">
              <div class="w-9 h-9 rounded-lg bg-violet-50 dark:bg-violet-900/30 flex items-center justify-center shrink-0">
                <iconify-icon icon="mdi:layers-outline" width="18" class="text-violet-600 dark:text-violet-400"></iconify-icon>
              </div>
              <span class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">{{ t('dashboard.card.installed') }}</span>
            </div>
            <p class="text-lg font-semibold text-gray-900 dark:text-gray-100">{{ toolchainCount }}</p>
          </div>

          <!-- Available Updates card -->
          <div class="bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-200 dark:border-gray-700 hover:shadow-sm transition-shadow">
            <template v-if="updateError">
              <div class="flex items-center gap-3 mb-3">
                <div class="w-9 h-9 rounded-lg bg-amber-50 dark:bg-amber-900/30 flex items-center justify-center shrink-0">
                  <iconify-icon icon="mdi:wifi-off" width="18" class="text-amber-600 dark:text-amber-400"></iconify-icon>
                </div>
                <span class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">{{ t('dashboard.card.updates') }}</span>
              </div>
              <p class="text-sm text-amber-600 dark:text-amber-400 leading-snug">{{ t('dashboard.status.networkError') }}</p>
            </template>
            <template v-else>
              <div class="flex items-center gap-3 mb-3">
                <div class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0" :class="updateCount > 0 ? 'bg-yellow-50 dark:bg-yellow-900/30' : 'bg-green-50 dark:bg-green-900/30'">
                  <iconify-icon :icon="updateCount > 0 ? 'mdi:bell-outline' : 'mdi:check-circle-outline'" width="18" :class="updateCount > 0 ? 'text-yellow-600 dark:text-yellow-400' : 'text-green-600 dark:text-green-400'"></iconify-icon>
                </div>
                <span class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">{{ t('dashboard.card.updates') }}</span>
              </div>
              <p class="text-lg font-semibold truncate" :class="updateCount > 0 ? 'text-yellow-600 dark:text-yellow-400' : 'text-green-600 dark:text-green-400'">
                {{ updateCount > 0 ? t('dashboard.status.updatesAvailable', { count: updateCount }) : t('dashboard.status.upToDate') }}
              </p>
            </template>
          </div>

          <!-- Environment card -->
          <div class="bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-200 dark:border-gray-700 hover:shadow-sm transition-shadow">
            <div class="flex items-center gap-3 mb-3">
              <div class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0" :class="envCheck?.rustup_installed ? 'bg-green-50 dark:bg-green-900/30' : 'bg-amber-50 dark:bg-amber-900/30'">
                <iconify-icon :icon="envCheck?.rustup_installed ? 'mdi:shield-check-outline' : 'mdi:alert-outline'" width="18" :class="envCheck?.rustup_installed ? 'text-green-600 dark:text-green-400' : 'text-amber-600 dark:text-amber-400'"></iconify-icon>
              </div>
              <span class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">{{ t('dashboard.card.environment') }}</span>
            </div>
            <p class="text-lg font-semibold truncate" :class="envCheck?.rustup_installed ? 'text-green-600 dark:text-green-400' : 'text-amber-600 dark:text-amber-400'">
              {{ envCheck?.rustup_installed ? t('dashboard.status.ready') : t('dashboard.rustNotFound') }}
            </p>
          </div>
        </div>
      </section>

      <!-- Version info + Quick links -->
      <div class="grid grid-cols-1 lg:grid-cols-5 gap-8">
        <!-- Version info -->
        <section v-if="versions" class="lg:col-span-3 space-y-4">
          <SectionTitle :title="t('dashboard.section.versions')" />
          <div class="space-y-3">
            <div class="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700 flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-lg bg-orange-50 dark:bg-orange-900/20 flex items-center justify-center">
                  <iconify-icon icon="mdi:cube-outline" width="16" class="text-orange-600 dark:text-orange-400"></iconify-icon>
                </div>
                <span class="text-sm text-gray-600 dark:text-gray-400">{{ t('dashboard.label.rustup') }}</span>
              </div>
              <span class="text-sm font-mono text-gray-900 dark:text-gray-100">{{ versions.rustup_version || t('common.status.na') }}</span>
            </div>
            <div class="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700 flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-lg bg-amber-50 dark:bg-amber-900/20 flex items-center justify-center">
                  <iconify-icon icon="mdi:package-variant-closed" width="16" class="text-amber-600 dark:text-amber-400"></iconify-icon>
                </div>
                <span class="text-sm text-gray-600 dark:text-gray-400">{{ t('dashboard.label.cargo') }}</span>
              </div>
              <span class="text-sm font-mono text-gray-900 dark:text-gray-100">{{ versions.cargo_version || t('common.status.na') }}</span>
            </div>
          </div>
        </section>

        <!-- Quick links -->
        <section class="lg:col-span-2 space-y-4">
          <SectionTitle :title="t('dashboard.section.quickActions')" />
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <router-link to="/updates" class="group bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-200 dark:border-gray-700 hover:border-sky-400 dark:hover:border-sky-600 hover:shadow-sm transition-all block">
              <div class="w-10 h-10 rounded-lg bg-green-50 dark:bg-green-900/30 flex items-center justify-center mb-3 group-hover:bg-green-100 dark:group-hover:bg-green-900/50 transition-colors">
                <iconify-icon icon="mdi:update" width="20" class="text-green-600 dark:text-green-400"></iconify-icon>
              </div>
              <h3 class="font-semibold text-gray-900 dark:text-gray-100 text-sm">{{ t('nav.updates') }}</h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 line-clamp-2">{{ t('dashboard.action.checkUpdates') }}</p>
            </router-link>
            <!-- Uninstall rustup -->
            <button v-if="envCheck?.rustup_installed" class="group bg-white dark:bg-gray-800 rounded-xl p-5 border border-gray-200 dark:border-gray-700 hover:border-red-400 dark:hover:border-red-600 hover:shadow-sm transition-all block text-left cursor-pointer" @click="triggerUninstall">
              <div class="w-10 h-10 rounded-lg bg-red-50 dark:bg-red-900/30 flex items-center justify-center mb-3 group-hover:bg-red-100 dark:group-hover:bg-red-900/50 transition-colors">
                <iconify-icon icon="mdi:delete-outline" width="20" class="text-red-500 dark:text-red-400"></iconify-icon>
              </div>
              <h3 class="font-semibold text-red-500 dark:text-red-400 text-sm">{{ t('app.uninstallRustup') }}</h3>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 line-clamp-2">{{ t('dashboard.action.uninstallRustupDesc') }}</p>
            </button>
          </div>
        </section>
      </div>
    </div>
  </PageLayout>
</template>
