<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref, computed, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import EmptyState from '../components/EmptyState.vue'
import PageLayout from '../components/PageLayout.vue'
import SectionTitle from '../components/SectionTitle.vue'
import ListItem from '../components/ListItem.vue'
import StatusBadge from '../components/StatusBadge.vue'
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
  updates.value.filter(u => u.toolchain === 'rustup' || u.toolchain.toLowerCase().includes('rustup'))
)

const toolchainUpdates = computed(() =>
  updates.value.filter(u => u.toolchain !== 'rustup' && !u.toolchain.toLowerCase().includes('rustup'))
)

function formatTitle(u: UpdateInfo): string {
  if (u.current_version && u.new_version) {
    return `${u.toolchain}  ${u.current_version} → ${u.new_version}`
  }
  if (u.current_version) {
    return `${u.toolchain}  ${u.current_version}`
  }
  return u.toolchain
}

async function checkUpdates() {
  loading.value = true
  try {
    updates.value = await doCheck()
  } catch {
    // ignore
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

  await listen<string>('update-log', event => {
    updateLogs.value.push(event.payload)
  })
})
</script>

<template>
  <PageLayout :group="t('nav.group.extend')" :title="t('updates.title')" :description="t('updates.description')">
    <template #actions>
      <BaseButton variant="secondary" :loading="updatingRustup" @click="updateRustup">
        {{ t('updates.action.updateRustup') }}
      </BaseButton>
      <BaseButton :loading="updating" @click="updateAll">
        {{ t('updates.action.updateAll') }}
      </BaseButton>
      <BaseButton variant="secondary" :loading="loading" @click="checkUpdates">
        {{ t('common.action.refresh') }}
      </BaseButton>
    </template>

    <!-- Loading state -->
    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('updates.status.checking') }}</div>

    <!-- Toolchain sections -->
    <template v-else>
      <!-- Rustup Update Section -->
      <div class="space-y-2">
        <SectionTitle :title="t('updates.section.rustupUpdate')" :count="rustupUpdates.length || undefined" />
        <div v-if="rustupUpdates.length === 0" class="text-sm text-gray-400 dark:text-gray-500 py-2">
          {{ t('updates.status.noRustupInfo') }}
        </div>
        <div v-else class="space-y-2">
          <ListItem v-for="u in rustupUpdates" :key="u.toolchain" :title="formatTitle(u)">
            <template #badges>
              <StatusBadge
                :type="u.up_to_date ? 'active' : 'updatable'"
                :label="
                  u.up_to_date ? t('updates.status.upToDate') : t('updates.status.updateAvailable', { suffix: '' })
                "
              />
            </template>
          </ListItem>
        </div>
      </div>

      <!-- Toolchain Updates Section -->
      <div class="space-y-2">
        <SectionTitle :title="t('updates.section.toolchainUpdates')" :count="toolchainUpdates.length || undefined" />
        <div v-if="toolchainUpdates.length === 0" class="text-sm text-gray-400 dark:text-gray-500 py-2">
          {{ t('updates.status.noToolchainUpdates') }}
        </div>
        <div v-else class="space-y-2">
          <ListItem v-for="u in toolchainUpdates" :key="u.toolchain" :title="formatTitle(u)">
            <template #badges>
              <StatusBadge
                :type="u.up_to_date ? 'active' : 'updatable'"
                :label="
                  u.up_to_date ? t('updates.status.upToDate') : t('updates.status.updateAvailable', { suffix: '' })
                "
              />
            </template>
          </ListItem>
        </div>
      </div>

      <EmptyState v-if="updates.length === 0" :message="t('updates.status.noToolchains')" />
    </template>

    <!-- Toolchain update progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="updateMode === 'all' ? t('updates.progress.updatingAllTitle') : t('updates.progress.updatingRustupTitle')"
      :status="updateStatus"
      :status-text="
        updateStatus === 'running'
          ? updateMode === 'all'
            ? t('updates.progress.updatingAllStatus')
            : t('updates.progress.updatingRustupStatus')
          : updateStatus === 'success'
          ? updateMode === 'all'
            ? t('updates.progress.allUpdated')
            : t('updates.progress.rustupUpdated')
          : t('updates.progress.failed')
      "
      :lines="updateLogs"
      @close="closeProgress"
    />
  </PageLayout>
</template>
