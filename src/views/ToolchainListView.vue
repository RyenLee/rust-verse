<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import EmptyState from '../components/EmptyState.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useRustup } from '../composables/useRustup'
import { useDataRefresh } from '../composables/useDataRefresh'
import { useToolchainOptions } from '../composables/useToolchainOptions'

const { t } = useI18n()
const {
  installToolchain: doInstall,
  uninstallToolchain: doUninstall,
  setDefaultToolchain: doSetDefault,
} = useRustup()
const { notifyToolchainChange } = useDataRefresh()
const { toolchains, loading, refresh } = useToolchainOptions()
const installing = ref(false)
const installLogs = ref<string[]>([])
const installStatus = ref<'running' | 'success' | 'error'>('running')
const showInstallPanel = ref(false)
const showProgress = ref(false)
const newChannel = ref('stable')
const newDate = ref('')
const confirmUninstall = ref<string | null>(null)

// Whether the selected channel requires a date
const channelRequiresDate = computed(() => {
  return newChannel.value === 'nightly'
})

// Available channel options with descriptions
const channelOptions = computed(() => [
  { value: 'stable', label: t('toolchains.channel.stable'), desc: t('toolchains.channel.stableDesc') },
  { value: 'beta', label: t('toolchains.channel.beta'), desc: t('toolchains.channel.betaDesc') },
  { value: 'nightly', label: t('toolchains.channel.nightly'), desc: t('toolchains.channel.nightlyDesc') },
])

async function installToolchain() {
  installing.value = true
  installLogs.value = []
  installStatus.value = 'running'
  showProgress.value = true
  try {
    const date = newChannel.value === 'nightly' && newDate.value ? newDate.value : undefined
    await doInstall(newChannel.value, date)
    installStatus.value = 'success'
    showInstallPanel.value = false
    newDate.value = ''
    notifyToolchainChange()
    await refresh()
  } catch (e) {
    installStatus.value = 'error'
    installLogs.value.push(`Error: ${e}`)
  } finally {
    installing.value = false
  }
}

async function uninstallToolchain(name: string) {
  try {
    await doUninstall(name)
    confirmUninstall.value = null
    notifyToolchainChange()
    await refresh()
  } catch {
    // ignore
  }
}

async function setDefault(name: string) {
  try {
    await doSetDefault(name)
    notifyToolchainChange()
    await refresh()
  } catch {
    // ignore
  }
}

function openInstallPanel() {
  newChannel.value = 'stable'
  newDate.value = ''
  showInstallPanel.value = true
}

function closeProgress() {
  showProgress.value = false
}

onMounted(async () => {
  // Initial toolchain load is handled by useToolchainOptions

  // Listen for streaming install logs
  await listen<string>('install-log', event => {
    installLogs.value.push(event.payload)
  })
  await listen('install-finished', () => {
    installing.value = false
  })
})
</script>

<template>
  <div class="p-6 space-y-4 h-full overflow-y-auto">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('toolchains.title') }}</h1>
      <BaseButton @click="openInstallPanel">
        {{ t('toolchains.action.installNew') }}
      </BaseButton>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>

    <!-- Toolchain list -->
    <div v-else class="space-y-2">
      <div
        v-for="tc in toolchains"
        :key="tc.name"
        class="bg-white dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700 flex items-center justify-between"
      >
        <div>
          <p class="font-medium text-gray-900 dark:text-gray-100">{{ tc.name }}</p>
          <div class="flex gap-2 mt-1">
            <span
              v-if="tc.is_default"
              class="text-xs bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300 px-2 py-0.5 rounded"
            >
              {{ t('common.status.default') }}
            </span>
            <span
              v-if="tc.is_active"
              class="text-xs bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 px-2 py-0.5 rounded"
            >
              {{ t('common.status.active') }}
            </span>
            <span class="text-xs bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400 px-2 py-0.5 rounded">
              {{ tc.channel }}
            </span>
          </div>
        </div>
        <div class="flex gap-2">
          <button
            v-if="!tc.is_default"
            class="text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-300 px-3 py-1.5 rounded transition-colors"
            @click="setDefault(tc.name)"
          >
            {{ t('toolchains.action.setDefault') }}
          </button>
          <button
            v-if="!tc.is_default"
            class="text-xs bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300 px-3 py-1.5 rounded transition-colors"
            @click="confirmUninstall = tc.name"
          >
            {{ t('common.action.uninstall') }}
          </button>
        </div>
      </div>

      <EmptyState v-if="toolchains.length === 0" :message="t('toolchains.status.noToolchains')" />
    </div>

    <!-- Install panel (slide-in from right) -->
    <Teleport to="body">
      <Transition name="slide-panel">
        <div v-if="showInstallPanel" class="fixed inset-0 z-50 flex justify-end">
          <!-- Backdrop -->
          <div class="absolute inset-0 bg-black/40" @click="showInstallPanel = false" />
          <!-- Panel -->
          <div
            class="relative w-full max-w-md bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 shadow-xl flex flex-col"
          >
            <!-- Header -->
            <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
              <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                {{ t('toolchains.dialog.installTitle') }}
              </h2>
              <button
                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                @click="showInstallPanel = false"
              >
                <iconify-icon icon="mdi:close" width="20"></iconify-icon>
              </button>
            </div>

            <!-- Body -->
            <div class="flex-1 overflow-y-auto p-6 space-y-5">
              <!-- Channel selection -->
              <div>
                <label class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 block">{{
                  t('toolchains.form.channel')
                }}</label>
                <div class="space-y-2">
                  <label
                    v-for="opt in channelOptions"
                    :key="opt.value"
                    :class="[
                      'flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors',
                      newChannel === opt.value
                        ? 'border-sky-500 bg-sky-50 dark:bg-sky-900/20'
                        : 'border-gray-200 dark:border-gray-600 hover:border-gray-300 dark:hover:border-gray-500',
                    ]"
                  >
                    <input v-model="newChannel" type="radio" :value="opt.value" class="mt-0.5 accent-sky-600" />
                    <div>
                      <p class="text-sm font-medium text-gray-900 dark:text-gray-100">{{ opt.label }}</p>
                      <p class="text-xs text-gray-500 dark:text-gray-400">{{ opt.desc }}</p>
                    </div>
                  </label>
                </div>
              </div>

              <!-- Date selection (only for nightly) -->
              <div v-if="channelRequiresDate">
                <label class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1 block">
                  {{ t('toolchains.form.date') }}
                  <span class="text-gray-400 font-normal">{{ t('toolchains.form.dateOptional') }}</span>
                </label>
                <p class="text-xs text-gray-500 dark:text-gray-400 mb-2">
                  {{ t('toolchains.help.dateHelp') }}
                </p>
                <input
                  v-model="newDate"
                  type="date"
                  class="w-full bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-200 rounded-md px-3 py-2 border border-gray-200 dark:border-gray-600"
                />
              </div>

              <!-- Info for stable/beta -->
              <div v-else class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3">
                <p class="text-xs text-gray-500 dark:text-gray-400">
                  {{
                    t('toolchains.help.stableBetaHelp', {
                      channel: newChannel === 'stable' ? t('toolchains.channel.stable') : t('toolchains.channel.beta'),
                    })
                  }}
                </p>
              </div>
            </div>

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
              <BaseButton variant="ghost" @click="showInstallPanel = false">
                {{ t('common.action.cancel') }}
              </BaseButton>
              <BaseButton :loading="installing" @click="installToolchain">
                {{ installing ? t('toolchains.progress.installing') : t('common.action.install') }}
              </BaseButton>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Uninstall confirm -->
    <ConfirmDialog
      v-if="confirmUninstall"
      :title="t('toolchains.dialog.confirmUninstall')"
      :message="t('toolchains.dialog.uninstallConfirm', { name: confirmUninstall })"
      :confirm-label="t('common.action.uninstall')"
      :danger="true"
      @confirm="uninstallToolchain(confirmUninstall!)"
      @cancel="confirmUninstall = null"
    />

    <!-- Install progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="t('toolchains.progress.title')"
      :status="installStatus"
      :status-text="
        installStatus === 'running'
          ? t('toolchains.progress.running', { channel: newChannel })
          : installStatus === 'success'
          ? t('toolchains.progress.success', { channel: newChannel })
          : t('toolchains.progress.failed')
      "
      :lines="installLogs"
      @close="closeProgress"
    />
  </div>
</template>

<style scoped>
.slide-panel-enter-active,
.slide-panel-leave-active {
  transition: opacity 0.2s ease;
}
.slide-panel-enter-active > div:last-child,
.slide-panel-leave-active > div:last-child {
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.slide-panel-enter-from,
.slide-panel-leave-to {
  opacity: 0;
}
.slide-panel-enter-from > div:last-child,
.slide-panel-leave-to > div:last-child {
  transform: translateX(100%);
}
</style>
