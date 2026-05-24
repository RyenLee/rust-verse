<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import EmptyState from '../components/EmptyState.vue'
import ListItem from '../components/ListItem.vue'
import PageLayout from '../components/PageLayout.vue'
import SectionTitle from '../components/SectionTitle.vue'
import ToolchainSelector from '../components/ToolchainSelector.vue'
import { useRustup, type OverrideInfo } from '../composables/useRustup'
import { useToolchainOptions } from '../composables/useToolchainOptions'
import { useResponsiveListHeight } from '../composables/useResponsiveListHeight'

const { t } = useI18n()
const { listOverrides, setOverride: doSet, removeOverride: doRemove } = useRustup()
const { toolchains } = useToolchainOptions()

const overrides = ref<OverrideInfo[]>([])
const loading = ref(true)
const dirPath = ref('')

// Responsive list height: Add override form(~160) + SectionTitle(~30)
const { listHeight } = useResponsiveListHeight({ aboveList: 190 })
const selectedToolchain = ref('')
const message = ref('')

async function browseDir() {
  const selected = await open({ directory: true, multiple: false })
  if (selected && typeof selected === 'string') {
    dirPath.value = selected
  }
}

async function refresh() {
  loading.value = true
  try {
    overrides.value = await listOverrides()
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
}

async function setOverride() {
  if (!dirPath.value || !selectedToolchain.value) return
  try {
    await doSet(dirPath.value, selectedToolchain.value)
    message.value = t('overrides.message.setSuccess', { dir: dirPath.value, toolchain: selectedToolchain.value })
    dirPath.value = ''
    await refresh()
  } catch (e) {
    message.value = t('overrides.message.setError', { error: String(e) })
  }
}

async function removeOverride(path: string) {
  try {
    await doRemove(path)
    message.value = t('overrides.message.removeSuccess', { path })
    await refresh()
  } catch (e) {
    message.value = t('overrides.message.removeError', { error: String(e) })
  }
}

onMounted(refresh)
</script>

<template>
  <PageLayout :group="t('nav.group.config')" :title="t('overrides.title')" :description="t('overrides.description')">
    <template #actions>
      <BaseButton variant="secondary" :loading="loading" @click="refresh">
        <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
        {{ t('common.action.refresh') }}
      </BaseButton>
    </template>

    <!-- Add override -->
    <SectionTitle :title="t('overrides.action.addOverride')" />
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 space-y-3">
      <div class="flex items-center gap-2">
        <input
          v-model="dirPath"
          :placeholder="t('overrides.placeholder.dirPath')"
          class="flex-1 min-w-0 bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-200 rounded-lg px-3 py-2 border border-gray-200 dark:border-gray-600 h-9 text-sm font-mono"
        />
        <BaseButton variant="secondary" @click="browseDir" class="h-9 shrink-0">
          {{ t('common.action.browse') }}
        </BaseButton>
      </div>
      <div class="flex items-center justify-between">
        <ToolchainSelector v-model="selectedToolchain" :toolchains="toolchains" />
        <BaseButton :disabled="!dirPath || !selectedToolchain" @click="setOverride" class="h-9">
          {{ t('common.action.set') }}
        </BaseButton>
      </div>
      <p v-if="message" class="text-sm text-gray-500 dark:text-gray-400">{{ message }}</p>
    </div>

    <!-- Override list -->
    <SectionTitle :title="t('overrides.title')" :count="overrides.length" class="mt-6" />
    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>
    <div v-else :style="{ maxHeight: listHeight }" class="overflow-y-auto scroll-container space-y-2 rounded-lg">
      <ListItem
        v-for="ov in overrides"
        :key="ov.path"
        :title="`📁 ${ov.path}`"
        :description="`→ ${ov.toolchain}`"
      >
        <template #actions>
          <button
            class="shrink-0 text-xs bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300 px-3 py-1.5 rounded transition-colors"
            @click="removeOverride(ov.path)"
          >
            {{ t('common.action.remove') }}
          </button>
        </template>
      </ListItem>

      <EmptyState v-if="overrides.length === 0" :message="t('overrides.status.noOverrides')" />
    </div>

    <!-- No-toolchain overlay -->
    <Teleport to="body">
      <div
        v-if="toolchains.length === 0"
        class="fixed inset-0 bg-white/80 dark:bg-gray-950/80 z-50 flex items-center justify-center"
      >
        <div class="flex flex-col items-center gap-4 text-center">
          <iconify-icon icon="mdi:cog-off-outline" width="40" class="text-gray-400 dark:text-gray-500"></iconify-icon>
          <p class="text-gray-500 dark:text-gray-400 text-sm">{{ t('toolchains.status.installFirst') }}</p>
          <router-link
            to="/toolchains"
            class="inline-flex items-center gap-2 px-4 py-2 bg-sky-600 hover:bg-sky-500 text-white text-sm font-medium rounded-lg transition-colors"
          >
            <iconify-icon icon="mdi:arrow-right" width="16"></iconify-icon>
            {{ t('toolchains.status.goInstall') }}
          </router-link>
        </div>
      </div>
    </Teleport>
  </PageLayout>
</template>
