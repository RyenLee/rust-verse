<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import PageLayout from '../components/PageLayout.vue'
import SectionTitle from '../components/SectionTitle.vue'
import { useEnvVars } from '../composables/useEnvVars'
import { useToast } from '../composables/useToast'
import { useTerminalReinit } from '../composables/useTerminalReinit'

const { t } = useI18n()
const { getEnvVar, setEnvVar, removeEnvVar } = useEnvVars()
const { success, error } = useToast()
const { reinitTerminal } = useTerminalReinit()

async function reinitTerminalSilent() {
  try {
    await reinitTerminal()
  } catch {
    // Terminal reinit is best-effort
  }
}

interface MirrorSource {
  id: string
  name: string
  dist_server: string
  update_root: string
  is_builtin: boolean
}

const BUILTIN_I18N_MAP: Record<string, string> = {
  'https://rsproxy.cn': 'rustupMirror.sources.rsproxy',
  'https://mirrors.ustc.edu.cn/rust-static': 'rustupMirror.sources.ustc',
  'https://mirrors.tuna.tsinghua.edu.cn/rustup': 'rustupMirror.sources.tuna',
  'https://mirrors.bfsu.edu.cn/rustup': 'rustupMirror.sources.bfsu',
  'https://mirrors.sjtug.sjtu.edu.cn/rust-static': 'rustupMirror.sources.sjtu',
  'https://mirrors.nju.edu.cn/rustup': 'rustupMirror.sources.nju',
  'https://mirrors.hust.edu.cn/rustup': 'rustupMirror.sources.hust',
}

function getDisplayName(source: MirrorSource): string {
  if (source.is_builtin) {
    return t(BUILTIN_I18N_MAP[source.dist_server] || source.name)
  }
  return source.name
}

const sources = ref<MirrorSource[]>([])
const currentDistServer = ref('')
const currentUpdateRoot = ref('')
const loading = ref(false)
const switching = ref('')

async function loadSources() {
  try {
    sources.value = await invoke<MirrorSource[]>('list_rustup_mirror_sources')
  } catch (e: any) {
    error(t('rustupMirror.message.loadFailed', { error: e?.message || String(e) }))
  }
}

async function loadCurrent() {
  loading.value = true
  try {
    const [dist, update] = await Promise.all([
      getEnvVar('RUSTUP_DIST_SERVER').catch(() => ({ name: 'RUSTUP_DIST_SERVER', value: '', is_set: false })),
      getEnvVar('RUSTUP_UPDATE_ROOT').catch(() => ({ name: 'RUSTUP_UPDATE_ROOT', value: '', is_set: false })),
    ])
    currentDistServer.value = dist.value
    currentUpdateRoot.value = update.value
  } catch (e: any) {
    error(t('rustupMirror.message.loadFailed', { error: e?.message || String(e) }))
  } finally {
    loading.value = false
  }
}

async function handleSwitch(source: MirrorSource) {
  switching.value = source.id
  try {
    await Promise.all([
      setEnvVar('RUSTUP_DIST_SERVER', source.dist_server),
      setEnvVar('RUSTUP_UPDATE_ROOT', source.update_root),
    ])
    currentDistServer.value = source.dist_server
    currentUpdateRoot.value = source.update_root
    reinitTerminalSilent()
    success(t('rustupMirror.message.switchSuccess', { name: getDisplayName(source) }))
  } catch (e: any) {
    error(t('rustupMirror.message.switchFailed', { error: e?.message || String(e) }))
  } finally {
    switching.value = ''
  }
}

async function handleDefault() {
  switching.value = 'default'
  try {
    await Promise.all([removeEnvVar('RUSTUP_DIST_SERVER'), removeEnvVar('RUSTUP_UPDATE_ROOT')])
    currentDistServer.value = ''
    currentUpdateRoot.value = ''
    reinitTerminalSilent()
    success(t('rustupMirror.message.defaultSuccess'))
  } catch (e: any) {
    error(t('rustupMirror.message.defaultFailed', { error: e?.message || String(e) }))
  } finally {
    switching.value = ''
  }
}

function isActive(source: MirrorSource): boolean {
  return currentDistServer.value === source.dist_server && currentUpdateRoot.value === source.update_root
}

// ── Add / Edit dialog state ──

const showFormDialog = ref(false)
const editingId = ref('')
const formName = ref('')
const formDistServer = ref('')
const formUpdateRoot = ref('')
const formSaving = ref(false)

function openAddForm() {
  editingId.value = ''
  formName.value = ''
  formDistServer.value = ''
  formUpdateRoot.value = ''
  showFormDialog.value = true
}

function openEditForm(source: MirrorSource) {
  editingId.value = source.id
  formName.value = source.name
  formDistServer.value = source.dist_server
  formUpdateRoot.value = source.update_root
  showFormDialog.value = true
}

function closeFormDialog() {
  showFormDialog.value = false
  editingId.value = ''
}

async function handleSave() {
  if (!formName.value.trim()) {
    error(t('rustupMirror.message.urlInvalid'))
    return
  }
  formSaving.value = true
  try {
    if (editingId.value) {
      await invoke<MirrorSource>('update_rustup_mirror_source', {
        id: editingId.value,
        name: formName.value.trim(),
        distServer: formDistServer.value.trim(),
        updateRoot: formUpdateRoot.value.trim(),
      })
      success(t('rustupMirror.message.editSuccess', { name: formName.value.trim() }))
    } else {
      await invoke<MirrorSource>('add_rustup_mirror_source', {
        name: formName.value.trim(),
        distServer: formDistServer.value.trim(),
        updateRoot: formUpdateRoot.value.trim(),
      })
      success(t('rustupMirror.message.addSuccess', { name: formName.value.trim() }))
    }
    closeFormDialog()
    await loadSources()
  } catch (e: any) {
    if (editingId.value) {
      error(t('rustupMirror.message.editFailed', { error: e?.message || String(e) }))
    } else {
      error(t('rustupMirror.message.addFailed', { error: e?.message || String(e) }))
    }
  } finally {
    formSaving.value = false
  }
}

// ── Delete with ConfirmDialog ──

const showDeleteConfirm = ref(false)
const deleteTarget = ref<MirrorSource | null>(null)
const deletingId = ref('')

function requestDelete(source: MirrorSource) {
  deleteTarget.value = source
  showDeleteConfirm.value = true
}

function cancelDelete() {
  showDeleteConfirm.value = false
  deleteTarget.value = null
}

async function confirmDelete() {
  if (!deleteTarget.value) return
  const source = deleteTarget.value
  deletingId.value = source.id
  try {
    await invoke('delete_rustup_mirror_source', { id: source.id })
    success(t('rustupMirror.message.deleteSuccess', { name: getDisplayName(source) }))
    await loadSources()
  } catch (e: any) {
    error(t('rustupMirror.message.deleteFailed', { error: e?.message || String(e) }))
  } finally {
    deletingId.value = ''
    showDeleteConfirm.value = false
    deleteTarget.value = null
  }
}

onMounted(async () => {
  await Promise.all([loadSources(), loadCurrent()])
})
</script>

<template>
  <PageLayout
    :group="t('nav.group.toolchain')"
    :title="t('rustupMirror.title')"
    :description="t('rustupMirror.description')"
  >
    <template #actions>
      <BaseButton variant="secondary" :loading="loading" @click="loadCurrent">
        <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
        {{ t('rustupMirror.action.refresh') }}
      </BaseButton>
      <BaseButton @click="openAddForm">
        <iconify-icon icon="mdi:plus" width="16"></iconify-icon>
        {{ t('rustupMirror.action.add') }}
      </BaseButton>
    </template>

    <!-- Current configuration -->
    <SectionTitle :title="t('rustupMirror.status.currentDistServer')" />
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 px-4 py-3 mb-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2 min-w-0">
          <iconify-icon icon="mdi:server" width="16" class="text-sky-500 shrink-0"></iconify-icon>
          <span class="text-sm font-medium text-gray-900 dark:text-gray-100 font-mono truncate">
            {{ currentDistServer || t('rustupMirror.status.notSet') }}
          </span>
        </div>
        <span v-if="!currentDistServer" class="text-xs text-gray-400 dark:text-gray-500 shrink-0 ml-2">
          {{ t('rustupMirror.status.official') }}
        </span>
      </div>
    </div>

    <SectionTitle :title="t('rustupMirror.status.currentUpdateRoot')" />
    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 px-4 py-3 mb-6">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2 min-w-0">
          <iconify-icon icon="mdi:cloud-sync" width="16" class="text-violet-500 shrink-0"></iconify-icon>
          <span class="text-sm font-medium text-gray-900 dark:text-gray-100 font-mono truncate">
            {{ currentUpdateRoot || t('rustupMirror.status.notSet') }}
          </span>
        </div>
        <span v-if="!currentUpdateRoot" class="text-xs text-gray-400 dark:text-gray-500 shrink-0 ml-2">
          {{ t('rustupMirror.status.official') }}
        </span>
      </div>
    </div>

    <!-- Mirror source table -->
    <SectionTitle :title="t('rustupMirror.field.source')" :count="sources.length" />
    <div class="overflow-hidden rounded-xl border border-gray-200 dark:border-gray-700">
      <div class="mirror-table-scroll max-h-[480px] overflow-y-auto">
        <table class="w-full text-sm">
          <thead class="sticky top-0 z-10">
            <tr class="bg-gray-50 dark:bg-gray-800/60">
              <th
                class="px-4 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider"
              >
                {{ t('rustupMirror.field.source') }}
              </th>
              <th
                class="px-4 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider hidden md:table-cell"
              >
                {{ t('rustupMirror.field.distServer') }}
              </th>
              <th
                class="px-4 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider hidden md:table-cell"
              >
                {{ t('rustupMirror.field.updateRoot') }}
              </th>
              <th
                class="px-4 py-3 text-right text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider"
              >
                {{ t('rustupMirror.field.actions') }}
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            <tr
              v-for="source in sources"
              :key="source.id"
              :class="[
                'bg-white dark:bg-gray-800 transition-colors',
                isActive(source) ? 'bg-sky-50 dark:bg-sky-900/10' : 'hover:bg-gray-50 dark:hover:bg-gray-750',
              ]"
            >
              <td class="px-4 py-3">
                <div class="flex items-center gap-2">
                  <iconify-icon
                    icon="mdi:check-circle"
                    width="16"
                    :class="isActive(source) ? 'text-emerald-500' : 'text-transparent'"
                  ></iconify-icon>
                  <span class="font-medium text-gray-900 dark:text-gray-100 whitespace-nowrap truncate">{{
                    getDisplayName(source)
                  }}</span>
                  <span
                    :class="
                      source.is_builtin
                        ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-800'
                        : 'bg-purple-50 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400 border-purple-200 dark:border-purple-800'
                    "
                    class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium border shrink-0"
                  >
                    {{ source.is_builtin ? t('rustupMirror.status.builtin') : t('rustupMirror.status.custom') }}
                  </span>
                </div>
                <div class="md:hidden mt-1 space-y-0.5">
                  <div class="text-xs text-gray-500 dark:text-gray-400 font-mono whitespace-nowrap truncate">
                    <span class="text-gray-400 dark:text-gray-500">DIST:</span> {{ source.dist_server }}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400 font-mono whitespace-nowrap truncate">
                    <span class="text-gray-400 dark:text-gray-500">ROOT:</span> {{ source.update_root }}
                  </div>
                </div>
              </td>
              <td class="px-4 py-3 hidden md:table-cell min-w-[150px] max-w-[400px]">
                <span class="text-xs text-gray-600 dark:text-gray-400 font-mono whitespace-nowrap truncate block">
                  {{ source.dist_server }}
                </span>
              </td>
              <td class="px-4 py-3 hidden md:table-cell min-w-[150px] max-w-[400px]">
                <span class="text-xs text-gray-600 dark:text-gray-400 font-mono whitespace-nowrap truncate block">
                  {{ source.update_root }}
                </span>
              </td>
              <td class="px-4 py-3 text-right">
                <div class="flex items-center justify-end gap-1">
                  <button
                    v-if="!isActive(source)"
                    class="inline-flex items-center justify-center w-8 h-8 rounded-lg text-sky-600 dark:text-sky-400 hover:bg-sky-50 dark:hover:bg-sky-900/30 transition-colors disabled:opacity-50"
                    :title="t('rustupMirror.action.switch')"
                    :disabled="!!switching"
                    @click="handleSwitch(source)"
                  >
                    <iconify-icon
                      :icon="switching === source.id ? 'mdi:loading' : 'mdi:swap-horizontal'"
                      :class="{ 'animate-spin': switching === source.id }"
                      width="16"
                    ></iconify-icon>
                  </button>
                  <span
                    v-else
                    class="inline-flex items-center justify-center w-8 h-8 rounded-lg text-emerald-600 dark:text-emerald-400"
                    :title="t('rustupMirror.status.current')"
                  >
                    <iconify-icon icon="mdi:check" width="16"></iconify-icon>
                  </span>

                  <button
                    class="inline-flex items-center justify-center w-8 h-8 rounded-lg text-amber-600 dark:text-amber-400 hover:bg-amber-50 dark:hover:bg-amber-900/30 transition-colors disabled:opacity-50"
                    :title="t('rustupMirror.action.edit')"
                    :disabled="!!switching"
                    @click="openEditForm(source)"
                  >
                    <iconify-icon icon="mdi:pencil" width="14"></iconify-icon>
                  </button>
                  <button
                    class="inline-flex items-center justify-center w-8 h-8 rounded-lg text-red-500 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors disabled:opacity-50"
                    :title="t('rustupMirror.action.delete')"
                    :disabled="!!deletingId"
                    @click="requestDelete(source)"
                  >
                    <iconify-icon
                      :icon="deletingId === source.id ? 'mdi:loading' : 'mdi:delete-outline'"
                      :class="{ 'animate-spin': deletingId === source.id }"
                      width="14"
                    ></iconify-icon>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Restore default -->
    <div class="mt-4">
      <BaseButton variant="danger" :loading="switching === 'default'" @click="handleDefault">
        <iconify-icon icon="mdi:restore" width="16"></iconify-icon>
        {{ t('rustupMirror.action.restoreDefault') }}
      </BaseButton>
    </div>

    <!-- Add / Edit Dialog -->
    <Teleport to="body">
      <Transition name="mirror-dialog">
        <div
          v-if="showFormDialog"
          class="fixed inset-0 z-50 flex items-center justify-center p-4"
          @click.self="closeFormDialog"
        >
          <div class="absolute inset-0 bg-black/50" @click="closeFormDialog" />
          <div
            class="relative bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-2xl w-full max-w-md"
          >
            <!-- Header -->
            <div class="flex items-center justify-between px-5 py-4 border-b border-gray-200 dark:border-gray-700">
              <div class="flex items-center gap-3">
                <div class="w-8 h-8 rounded-lg flex items-center justify-center bg-sky-50 dark:bg-sky-900/30">
                  <iconify-icon
                    :icon="editingId ? 'mdi:pencil' : 'mdi:plus'"
                    width="18"
                    class="text-sky-600 dark:text-sky-400"
                  ></iconify-icon>
                </div>
                <h2 class="text-base font-semibold text-gray-900 dark:text-gray-100">
                  {{ editingId ? t('rustupMirror.action.edit') : t('rustupMirror.action.add') }}
                </h2>
              </div>
              <button
                class="p-1 rounded-md text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                @click="closeFormDialog"
              >
                <iconify-icon icon="mdi:close" width="18"></iconify-icon>
              </button>
            </div>

            <!-- Form body -->
            <div class="px-5 py-4 space-y-4">
              <div>
                <label class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1.5">
                  {{ t('rustupMirror.field.name') }}
                </label>
                <input
                  v-model="formName"
                  type="text"
                  :placeholder="t('rustupMirror.field.name')"
                  class="w-full h-9 px-3 rounded-lg text-sm bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500/30 focus:border-sky-500 transition-colors"
                />
              </div>
              <div>
                <label class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1.5">
                  {{ t('rustupMirror.field.distServer') }}
                </label>
                <input
                  v-model="formDistServer"
                  type="url"
                  :placeholder="t('rustupMirror.placeholder.distServer')"
                  class="w-full h-9 px-3 rounded-lg text-sm font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500/30 focus:border-sky-500 transition-colors"
                />
              </div>
              <div>
                <label class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1.5">
                  {{ t('rustupMirror.field.updateRoot') }}
                </label>
                <input
                  v-model="formUpdateRoot"
                  type="url"
                  :placeholder="t('rustupMirror.placeholder.updateRoot')"
                  class="w-full h-9 px-3 rounded-lg text-sm font-mono bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500/30 focus:border-sky-500 transition-colors"
                />
              </div>
            </div>

            <!-- Footer -->
            <div class="flex justify-end gap-2 px-5 py-3 border-t border-gray-200 dark:border-gray-700">
              <BaseButton variant="ghost" :disabled="formSaving" @click="closeFormDialog">
                {{ t('rustupMirror.action.cancel') }}
              </BaseButton>
              <BaseButton :loading="formSaving" @click="handleSave">
                <iconify-icon icon="mdi:content-save" width="16"></iconify-icon>
                {{ t('rustupMirror.action.save') }}
              </BaseButton>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <!-- Delete Confirm Dialog -->
    <ConfirmDialog
      :visible="showDeleteConfirm"
      :title="t('rustupMirror.action.confirmDelete')"
      :message="t('rustupMirror.message.deleteConfirm', { name: deleteTarget ? getDisplayName(deleteTarget) : '' })"
      :confirm-label="t('rustupMirror.action.delete')"
      danger
      :loading="!!deletingId"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </PageLayout>
</template>

<style scoped>
.mirror-dialog-enter-active {
  transition: opacity 0.2s ease;
}
.mirror-dialog-enter-active > div:last-child {
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.2s ease;
}
.mirror-dialog-leave-active {
  transition: opacity 0.15s ease;
}
.mirror-dialog-enter-from,
.mirror-dialog-leave-to {
  opacity: 0;
}
.mirror-dialog-enter-from > div:last-child {
  transform: scale(0.95);
  opacity: 0;
}
</style>

<style>
.mirror-table-scroll::-webkit-scrollbar {
  width: 6px;
}
.mirror-table-scroll::-webkit-scrollbar-track {
  background: transparent;
}
.mirror-table-scroll::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.4);
  border-radius: 3px;
}
.mirror-table-scroll::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.6);
}
.dark .mirror-table-scroll::-webkit-scrollbar-thumb {
  background-color: rgba(75, 85, 99, 0.5);
}
.dark .mirror-table-scroll::-webkit-scrollbar-thumb:hover {
  background-color: rgba(75, 85, 99, 0.7);
}
</style>
