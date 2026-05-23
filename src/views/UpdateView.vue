<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref, computed, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import EmptyState from '../components/EmptyState.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useRustup, type UpdateInfo } from '../composables/useRustup'
import { useDataRefresh } from '../composables/useDataRefresh'

const { t } = useI18n()
const { checkUpdate: doCheck, updateAll: doUpdateAll, updateRustup: doUpdateRustup } = useRustup()
const { notifyToolchainChange } = useDataRefresh()

const updates = ref<UpdateInfo[]>([])
const loading = ref(true)
const updating = ref(false)
const updatingRustup = ref(false)
const updateLogs = ref<string[]>([])
const updateStatus = ref<'running' | 'success' | 'error'>('running')
const updateMode = ref<'all' | 'rustup'>('all')
const showProgress = ref(false)

// Separate rustup updates from other toolchain updates
const rustupUpdates = computed(() =>
  updates.value.filter((u) => u.toolchain === 'rustup' || u.toolchain.toLowerCase().includes('rustup'))
)

const toolchainUpdates = computed(() =>
  updates.value.filter((u) => u.toolchain !== 'rustup' && !u.toolchain.toLowerCase().includes('rustup'))
)

async function checkUpdates() {
  loading.value = true
  try {
    updates.value = await doCheck()
  } catch (e) {
    console.error('Failed to check updates:', e)
  } finally {
    loading.value = false
  }
}

async function updateAll() {
  updating.value = true
  updateLogs.value = []
  updateStatus.value = 'running'
  updateMode.value = 'all'
  showProgress.value = true
  try {
    await doUpdateAll()
    updateStatus.value = 'success'
    notifyToolchainChange()
    // Re-check updates after a successful update to reflect new state
    await checkUpdates()
  } catch (e) {
    updateStatus.value = 'error'
    updateLogs.value.push(`Error: ${e}`)
  } finally {
    updating.value = false
  }
}

async function updateRustup() {
  updatingRustup.value = true
  updateLogs.value = []
  updateStatus.value = 'running'
  updateMode.value = 'rustup'
  showProgress.value = true
  try {
    await doUpdateRustup()
    updateStatus.value = 'success'
    notifyToolchainChange()
    // Re-check updates after rustup self-update to reflect new state
    await checkUpdates()
  } catch (e) {
    updateStatus.value = 'error'
    updateLogs.value.push(`Error: ${e}`)
  } finally {
    updatingRustup.value = false
  }
}

function closeProgress() {
  showProgress.value = false
}

onMounted(async () => {
  await checkUpdates()

  await listen<string>('update-log', (event) => {
    updateLogs.value.push(event.payload)
  })
})
</script>

<template>
  <div class="p-6 space-y-6 h-full overflow-y-auto">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('updates.title') }}</h1>
      <div class="flex gap-2">
        <BaseButton variant="secondary" :loading="updatingRustup" @click="updateRustup">
          {{ t('updates.action.updateRustup') }}
        </BaseButton>
        <BaseButton :loading="updating" @click="updateAll">
          {{ t('updates.action.updateAll') }}
        </BaseButton>
      </div>
    </div>

    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('updates.status.checking') }}</div>

    <template v-else>
      <!-- Rustup Update Section -->
      <div>
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3">{{ t('updates.section.rustupUpdate') }}</h2>
        <div v-if="rustupUpdates.length === 0" class="text-sm text-gray-400 dark:text-gray-500 py-2">
          {{ t('updates.status.noRustupInfo') }}
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="u in rustupUpdates"
            :key="u.toolchain"
            class="bg-white dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700 flex items-center justify-between"
          >
            <div>
              <p class="text-gray-900 dark:text-gray-200 truncate">{{ u.toolchain }}</p>
              <p v-if="u.current_version" class="text-sm text-gray-500 dark:text-gray-400 mt-1 truncate">
                {{ u.current_version }}
              </p>
            </div>
            <span
              :class="[
                'text-xs px-3 py-1 rounded max-w-[200px] truncate shrink-0',
                u.up_to_date
                  ? 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                  : 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300',
              ]"
            >
              {{ u.up_to_date ? t('updates.status.upToDate') : t('updates.status.updateAvailable', { suffix: u.new_version ? ': ' + u.new_version : '' }) }}
            </span>
          </div>
        </div>
      </div>

      <!-- Toolchain Updates Section -->
      <div>
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3">{{ t('updates.section.toolchainUpdates') }}</h2>
        <div v-if="toolchainUpdates.length === 0" class="text-sm text-gray-400 dark:text-gray-500 py-2">
          {{ t('updates.status.noToolchainUpdates') }}
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="u in toolchainUpdates"
            :key="u.toolchain"
            class="bg-white dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700 flex items-center justify-between"
          >
            <div>
              <p class="text-gray-900 dark:text-gray-200 truncate">{{ u.toolchain }}</p>
              <p v-if="u.current_version" class="text-sm text-gray-500 dark:text-gray-400 mt-1 truncate">
                {{ u.current_version }}
              </p>
            </div>
            <span
              :class="[
                'text-xs px-3 py-1 rounded max-w-[200px] truncate shrink-0',
                u.up_to_date
                  ? 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                  : 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300',
              ]"
            >
              {{ u.up_to_date ? t('updates.status.upToDate') : t('updates.status.updateAvailable', { suffix: u.new_version ? ': ' + u.new_version : '' }) }}
            </span>
          </div>
        </div>
      </div>

      <EmptyState v-if="updates.length === 0" :message="t('updates.status.noToolchains')" />
    </template>

    <!-- Update progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="updateMode === 'all' ? t('updates.progress.updatingAllTitle') : t('updates.progress.updatingRustupTitle')"
      :status="updateStatus"
      :status-text="updateStatus === 'running' ? (updateMode === 'all' ? t('updates.progress.updatingAllStatus') : t('updates.progress.updatingRustupStatus')) : updateStatus === 'success' ? (updateMode === 'all' ? t('updates.progress.allUpdated') : t('updates.progress.rustupUpdated')) : t('updates.progress.failed')"
      :lines="updateLogs"
      @close="closeProgress"
    />
  </div>
</template>
