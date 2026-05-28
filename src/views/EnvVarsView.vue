<script setup lang="ts">
import { onMounted, ref, computed, reactive } from 'vue'
import PageLayout from '../components/PageLayout.vue'
import SearchInput from '../components/SearchInput.vue'
import SectionTitle from '../components/SectionTitle.vue'
import ListItem from '../components/ListItem.vue'
import StatusBadge from '../components/StatusBadge.vue'
import BaseButton from '../components/BaseButton.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { useEnvVars, type EnvVarInfo } from '../composables/useEnvVars'
import { usePersist } from '../composables/usePersist'
import { useResponsiveListHeight } from '../composables/useResponsiveListHeight'
import { useToast } from '../composables/useToast'
import { useDataRefresh } from '../composables/useDataRefresh'
import { useTerminalReinit } from '../composables/useTerminalReinit'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const { listEnvVars, setEnvVar, removeEnvVar, updateEnvVarMeta, deleteEnvVarMeta } = useEnvVars()
const { persistEnvVar, removePersistedEnvVar, listPersistedEnvVars } = usePersist()
const { success, error } = useToast()
const { notifyEnvVarChange } = useDataRefresh()
const { reinitTerminal } = useTerminalReinit()

// Best-effort terminal reinit — silently catches errors
async function reinitTerminalSilent() {
  try {
    await reinitTerminal()
  } catch {
    // Terminal reinit is best-effort
  }
}

const envVars = ref<EnvVarInfo[]>([])
const loading = ref(true)
const searchQuery = ref('')
const activeCategory = ref('all')

// Dismissible warning banner
const WARNING_KEY = 'envVars-warning-dismissed'
const warningDismissed = ref(localStorage.getItem(WARNING_KEY) === 'true')
function dismissWarning() {
  warningDismissed.value = true
  localStorage.setItem(WARNING_KEY, 'true')
}

// Edit modal state
const showEditModal = ref(false)
const editingIdx = ref<number | null>(null)
const editForm = reactive({
  category: '',
  name: '',
  description: '',
  rec: '',
  def: '',
  notes: '',
})

// View modal state
const viewVar = ref<EnvVarInfo | null>(null)

// Delete confirmation state
const confirmDelete = ref<EnvVarInfo | null>(null)

// Critical variable confirmation state (CARGO_HOME only - requires checkbox)
const CRITICAL_VARS = new Set(['CARGO_HOME'])

// Normal variable confirmation state (all vars - simple confirm)
const normalConfirm = ref<{
  type: 'apply' | 'deactivate'
  variable: EnvVarInfo
} | null>(null)
const criticalConfirm = ref<{
  type: 'apply' | 'deactivate'
  variable: EnvVarInfo
} | null>(null)
const criticalConfirmed = ref(false)

// Persisted state tracking
const persistedVars = ref<Set<string>>(new Set())

// Category definitions with icons and colors
const CATEGORY_KEYS = ['paths_cache', 'network_proxy', 'build_perf', 'debug_diag', 'misc'] as const

const categoryIcons: Record<string, string> = {
  paths_cache: 'mdi:folder-outline',
  network_proxy: 'mdi:web',
  build_perf: 'mdi:lightning-bolt',
  debug_diag: 'mdi:bug-outline',
  misc: 'mdi:dots-horizontal-circle-outline',
}

const categoryColors: Record<string, { bg: string; text: string; border: string }> = {
  paths_cache: {
    bg: 'bg-sky-50 dark:bg-sky-900/30',
    text: 'text-sky-700 dark:text-sky-300',
    border: 'border-sky-200 dark:border-sky-800',
  },
  network_proxy: {
    bg: 'bg-violet-50 dark:bg-violet-900/30',
    text: 'text-violet-700 dark:text-violet-300',
    border: 'border-violet-200 dark:border-violet-800',
  },
  build_perf: {
    bg: 'bg-amber-50 dark:bg-amber-900/30',
    text: 'text-amber-700 dark:text-amber-300',
    border: 'border-amber-200 dark:border-amber-800',
  },
  debug_diag: {
    bg: 'bg-rose-50 dark:bg-rose-900/30',
    text: 'text-rose-700 dark:text-rose-300',
    border: 'border-rose-200 dark:border-rose-800',
  },
  misc: {
    bg: 'bg-emerald-50 dark:bg-emerald-900/30',
    text: 'text-emerald-700 dark:text-emerald-300',
    border: 'border-emerald-200 dark:border-emerald-800',
  },
}

const filteredVars = computed(() => {
  let list = envVars.value
  if (activeCategory.value !== 'all') {
    list = list.filter(v => v.category === activeCategory.value)
  }
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter(
      v =>
        v.name.toLowerCase().includes(q) ||
        v.description.toLowerCase().includes(q) ||
        (v.notes && v.notes.toLowerCase().includes(q))
    )
  }
  return list
})

// Responsive list height: filters area(90) + SectionTitle(~30)
const { listHeight } = useResponsiveListHeight({ filters: 90, aboveList: 30 })

function getCategoryLabel(key: string): string {
  const catKey = `envVars.category.${key}` as const
  const result = t(catKey)
  // If the key doesn't exist in locale, fall back to the key itself
  return result === catKey ? key : result
}

async function loadData() {
  loading.value = true
  try {
    envVars.value = await listEnvVars()
  } catch (e) {
    error(t('envVars.message.loadFailed'))
  } finally {
    loading.value = false
  }
}

async function loadPersistedStatus() {
  try {
    const list = await listPersistedEnvVars()
    persistedVars.value = new Set(list)
  } catch {
    // Silently ignore - persisted status is optional
  }
}

// View variable details
function openView(v: EnvVarInfo) {
  viewVar.value = v
}

// Edit / Add modal
function openAddModal() {
  editingIdx.value = null
  editForm.category = activeCategory.value !== 'all' ? activeCategory.value : CATEGORY_KEYS[0]
  editForm.name = ''
  editForm.description = ''
  editForm.rec = ''
  editForm.def = ''
  editForm.notes = ''
  showEditModal.value = true
}

// Track the original values for detecting renames / category changes
const editOriginal = reactive({
  category: '',
  name: '',
})

function openEditModal(v: EnvVarInfo, idx: number) {
  editingIdx.value = idx
  editForm.category = v.category
  editForm.name = v.name
  editForm.description = v.description
  editForm.rec = v.rec ?? ''
  editForm.def = v.def ?? ''
  editForm.notes = v.notes ?? ''
  editOriginal.category = v.category
  editOriginal.name = v.name
  showEditModal.value = true
}

function closeEditModal() {
  showEditModal.value = false
}

async function saveEditModal() {
  const name = editForm.name.trim()
  if (!name) {
    error(t('envVars.message.nameEmpty'))
    return
  }
  if (name.includes('=') || name.includes('\0')) {
    error(t('envVars.message.nameInvalid'))
    return
  }

  try {
    // 1. Update metadata in database (always)
    const isEditing = editingIdx.value !== null
    await updateEnvVarMeta({
      category: editForm.category,
      name,
      description: editForm.description,
      rec: editForm.rec || null,
      def: editForm.def || null,
      notes: editForm.notes,
      oldCategory: isEditing ? editOriginal.category : undefined,
      oldName: isEditing ? editOriginal.name : undefined,
    })

    // 2. Check if variable is active (persisted to system)
    // Use the original name for the check since the new name hasn't been persisted yet
    const checkName = isEditing ? editOriginal.name : name
    const isActive = persistedVars.value.has(checkName)

    if (isActive) {
      // Active: update process env var AND re-persist to system
      const value = editForm.rec || editForm.def || ''
      if (value) {
        await setEnvVar(name, value)
        await persistEnvVar(name, value)
        reinitTerminalSilent()
      }
      // If the variable was renamed, remove the old one from system
      if (isEditing && editOriginal.name !== name) {
        await removePersistedEnvVar(editOriginal.name)
        await removeEnvVar(editOriginal.name)
        persistedVars.value.delete(editOriginal.name)
      }
      persistedVars.value.add(name)
      persistedVars.value = new Set(persistedVars.value)
      success(t('envVars.message.setSuccessActive', { name }))
    } else {
      // Inactive: only database updated, no process/system changes
      success(t('envVars.message.setSuccessInactive', { name }))
    }

    showEditModal.value = false
    notifyEnvVarChange()
    await loadData()
  } catch (e: any) {
    error(t('envVars.message.setFailed', { error: String(e) }))
  }
}

// Apply: set the configured value to process AND persist to system
async function applyVar(v: EnvVarInfo) {
  if (!v.rec) {
    error(t('envVars.message.noValueToApply'))
    return
  }
  // Critical variable (CARGO_HOME): show full confirmation dialog with checkbox
  if (CRITICAL_VARS.has(v.name)) {
    criticalConfirm.value = { type: 'apply', variable: v }
    criticalConfirmed.value = false
    return
  }
  // All other variables: show simple confirmation dialog
  normalConfirm.value = { type: 'apply', variable: v }
  return
}

async function doApplyVar(v: EnvVarInfo) {
  try {
    // 1. Set in current process
    await setEnvVar(v.name, v.rec)
    // 2. Persist to system
    await persistEnvVar(v.name, v.rec)
    persistedVars.value.add(v.name)
    persistedVars.value = new Set(persistedVars.value)
    reinitTerminalSilent()
    success(t('envVars.message.applySuccess', { name: v.name, value: v.rec }))
    notifyEnvVarChange()
    await loadData()
  } catch (e: any) {
    error(t('envVars.message.applyFailed', { error: String(e) }))
  }
}

// Deactivate: remove from system persistence AND unset from current process
async function deactivateVar(v: EnvVarInfo) {
  // Critical variable (CARGO_HOME): show full confirmation dialog with checkbox
  if (CRITICAL_VARS.has(v.name)) {
    criticalConfirm.value = { type: 'deactivate', variable: v }
    criticalConfirmed.value = false
    return
  }
  // All other variables: show simple confirmation dialog
  normalConfirm.value = { type: 'deactivate', variable: v }
  return
}

async function doDeactivateVar(v: EnvVarInfo) {
  try {
    // 1. Remove from system persistence (registry / shell config)
    await removePersistedEnvVar(v.name)
    // 2. Remove from current process
    await removeEnvVar(v.name)
    // 3. Update local tracking
    persistedVars.value.delete(v.name)
    persistedVars.value = new Set(persistedVars.value)
    reinitTerminalSilent()
    success(t('envVars.message.deactivateSuccess', { name: v.name }))
    notifyEnvVarChange()
    await loadData()
  } catch (e: any) {
    error(t('envVars.message.deactivateFailed', { error: String(e) }))
  }
}

// Handle critical variable confirmation
function handleCriticalConfirm() {
  if (!criticalConfirm.value) return
  const { type, variable } = criticalConfirm.value
  criticalConfirm.value = null
  if (type === 'apply') {
    doApplyVar(variable)
  } else {
    doDeactivateVar(variable)
  }
}

// Handle normal variable confirmation
function handleNormalConfirm() {
  if (!normalConfirm.value) return
  const { type, variable } = normalConfirm.value
  normalConfirm.value = null
  if (type === 'apply') {
    doApplyVar(variable)
  } else {
    doDeactivateVar(variable)
  }
}

// Delete: remove from database only (does NOT affect system environment variables)
async function deleteVar(v: EnvVarInfo) {
  try {
    await deleteEnvVarMeta(v.category, v.name)
    confirmDelete.value = null
    success(t('envVars.dialog.deleteSuccess', { name: v.name }))
    notifyEnvVarChange()
    await loadData()
  } catch (e: any) {
    error(t('envVars.message.deleteFailed', { error: String(e) }))
  }
}

onMounted(async () => {
  await Promise.all([loadData(), loadPersistedStatus()])
})
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden">
    <PageLayout :group="t('nav.group.config')" :title="t('envVars.title')" :description="t('envVars.description')">
      <template #actions>
        <BaseButton variant="secondary" :loading="loading" @click="loadData">
          <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
          {{ t('common.action.refresh') }}
        </BaseButton>
        <BaseButton @click="openAddModal">
          <iconify-icon icon="mdi:plus" width="16"></iconify-icon>
          {{ t('envVars.action.addVariable') }}
        </BaseButton>
      </template>

      <template #filters>
        <div class="w-full space-y-3">
          <!-- Search -->
          <SearchInput v-model="searchQuery" :placeholder="t('envVars.placeholder.search')" />

          <!-- Category tabs -->
          <div class="flex gap-1 overflow-x-auto pb-1 scroll-container-x">
            <button
              class="shrink-0 h-8 px-3 rounded-lg text-sm font-medium transition-colors inline-flex items-center gap-1.5 border"
              :class="
                activeCategory === 'all'
                  ? 'bg-sky-100 dark:bg-sky-900/40 text-sky-700 dark:text-sky-300 border-sky-200 dark:border-sky-800'
                  : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-750'
              "
              @click="activeCategory = 'all'"
            >
              <iconify-icon icon="mdi:format-list-bulleted" width="14"></iconify-icon>
              {{ t('envVars.status.all') }}
            </button>
            <button
              v-for="cat in CATEGORY_KEYS"
              :key="cat"
              class="shrink-0 h-8 px-3 rounded-lg text-sm font-medium transition-colors inline-flex items-center gap-1.5 border whitespace-nowrap"
              :class="
                activeCategory === cat
                  ? [categoryColors[cat]?.bg, categoryColors[cat]?.text, categoryColors[cat]?.border]
                  : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-750'
              "
              @click="activeCategory = cat"
            >
              <iconify-icon :icon="categoryIcons[cat]" width="14"></iconify-icon>
              {{ getCategoryLabel(cat) }}
            </button>
          </div>

          <!-- Warning: Apply will modify system env vars (dismissible) -->
          <div
            v-if="!warningDismissed"
            class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg px-3 py-2.5 flex items-start gap-2"
          >
            <iconify-icon
              icon="mdi:alert-octagon-outline"
              width="16"
              class="text-red-500 dark:text-red-400 mt-0.5 shrink-0"
            ></iconify-icon>
            <p class="text-sm text-red-700 dark:text-red-300 flex-1">{{ t('envVars.warning') }}</p>
            <button
              class="shrink-0 text-red-400 hover:text-red-600 dark:hover:text-red-300 transition-colors"
              :title="t('common.action.close')"
              @click="dismissWarning"
            >
              <iconify-icon icon="mdi:close" width="16"></iconify-icon>
            </button>
          </div>
        </div>
      </template>

      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center py-16">
        <div class="text-center space-y-3">
          <div class="inline-flex items-center justify-center w-12 h-12 rounded-full bg-sky-50 dark:bg-sky-900/30">
            <svg
              class="animate-spin h-6 w-6 text-sky-600 dark:text-sky-400"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
          </div>
          <p class="text-gray-500 dark:text-gray-400 text-sm">{{ t('common.status.loading') }}</p>
        </div>
      </div>

      <!-- Card/list layout -->
      <div v-else>
        <div v-if="filteredVars.length === 0" class="flex flex-col items-center justify-center py-16 text-center">
          <div class="w-14 h-14 rounded-2xl bg-gray-50 dark:bg-gray-800 flex items-center justify-center mb-3">
            <iconify-icon icon="mdi:variable" width="28" class="text-gray-400"></iconify-icon>
          </div>
          <p class="text-gray-500 dark:text-gray-400 text-sm">
            {{ searchQuery ? t('envVars.status.noMatching') : t('envVars.status.noVars') }}
          </p>
        </div>

        <div v-else class="space-y-2">
          <SectionTitle :title="t('envVars.field.variable')" :count="filteredVars.length" />
          <div :style="{ maxHeight: listHeight }" class="overflow-y-auto scroll-container space-y-2 rounded-lg">
            <ListItem
              v-for="(v, idx) in filteredVars"
              :key="v.name"
              :title="v.name"
              :description="v.description"
              :active="v.is_set"
              :class="[CRITICAL_VARS.has(v.name) ? 'border-amber-300 dark:border-amber-700' : '']"
            >
              <template #title>
                <span
                  class="font-mono text-sm font-semibold"
                  :class="
                    CRITICAL_VARS.has(v.name)
                      ? 'text-amber-700 dark:text-amber-300'
                      : 'text-gray-900 dark:text-gray-100'
                  "
                >
                  {{ v.name }}
                </span>
              </template>
              <template #badges>
                <StatusBadge v-if="CRITICAL_VARS.has(v.name)" type="danger" :label="t('envVars.criticalVar.title')" />
                <StatusBadge v-if="persistedVars.has(v.name)" type="active" :label="t('envVars.persist.label')" />
                <StatusBadge v-if="v.is_set" type="installed" :label="t('envVars.status.set')" />
              </template>
              <template #actions>
                <button
                  class="inline-flex items-center justify-center w-7 h-7 rounded-md text-sky-600 dark:text-sky-400 hover:bg-sky-50 dark:hover:bg-sky-900/30 transition-colors"
                  :title="t('envVars.action.view')"
                  @click="openView(v)"
                >
                  <iconify-icon icon="mdi:eye-outline" width="14"></iconify-icon>
                </button>
                <button
                  v-if="!persistedVars.has(v.name)"
                  class="inline-flex items-center justify-center w-7 h-7 rounded-md text-emerald-600 dark:text-emerald-400 hover:bg-emerald-50 dark:hover:bg-emerald-900/30 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                  :title="t('envVars.action.apply')"
                  :disabled="!v.rec"
                  @click="applyVar(v)"
                >
                  <iconify-icon icon="mdi:check-circle-outline" width="14"></iconify-icon>
                </button>
                <button
                  v-else
                  class="inline-flex items-center justify-center w-7 h-7 rounded-md text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                  :title="t('envVars.action.deactivate')"
                  @click="deactivateVar(v)"
                >
                  <iconify-icon icon="mdi:close-circle-outline" width="14"></iconify-icon>
                </button>
                <button
                  class="inline-flex items-center justify-center w-7 h-7 rounded-md text-amber-600 dark:text-amber-400 hover:bg-amber-50 dark:hover:bg-amber-900/30 transition-colors"
                  :title="t('envVars.action.edit')"
                  @click="openEditModal(v, idx)"
                >
                  <iconify-icon icon="mdi:pencil-outline" width="14"></iconify-icon>
                </button>
                <button
                  class="inline-flex items-center justify-center w-7 h-7 rounded-md text-red-500 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                  :title="t('envVars.action.delete')"
                  @click="confirmDelete = v"
                >
                  <iconify-icon icon="mdi:delete-outline" width="14"></iconify-icon>
                </button>
              </template>
            </ListItem>
          </div>
        </div>
      </div>
    </PageLayout>

    <!-- View modal -->
    <Teleport to="body">
      <Transition name="dialog-overlay">
        <div
          v-if="viewVar"
          class="fixed inset-0 bg-black/40 backdrop-blur-[2px] flex items-center justify-center z-50"
          @click.self="viewVar = null"
        >
          <Transition name="dialog-panel" appear>
            <div
              class="bg-white dark:bg-gray-800 rounded-2xl w-full max-w-xl border border-gray-200 dark:border-gray-700 shadow-2xl"
            >
              <!-- Header -->
              <div class="px-6 pt-5 pb-4 border-b border-gray-100 dark:border-gray-700/50">
                <div class="flex items-center gap-3">
                  <div
                    class="w-9 h-9 rounded-xl bg-sky-50 dark:bg-sky-900/30 flex items-center justify-center shrink-0"
                  >
                    <iconify-icon icon="mdi:variable" width="20" class="text-sky-600 dark:text-sky-400"></iconify-icon>
                  </div>
                  <div>
                    <h2 class="text-base font-semibold text-gray-900 dark:text-gray-100">
                      {{ t('envVars.dialog.viewTitle') }}
                    </h2>
                    <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 font-mono">{{ viewVar.name }}</p>
                  </div>
                </div>
              </div>

              <!-- Body -->
              <div class="px-6 py-5 space-y-4">
                <div>
                  <label
                    class="block text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1"
                    >{{ t('envVars.form.category') }}</label
                  >
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-md text-xs font-semibold"
                    :class="[categoryColors[viewVar.category]?.bg, categoryColors[viewVar.category]?.text]"
                  >
                    {{ getCategoryLabel(viewVar.category) }}
                  </span>
                </div>
                <div>
                  <label
                    class="block text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1"
                    >{{ t('envVars.field.description') }}</label
                  >
                  <p class="text-sm text-gray-700 dark:text-gray-300 break-words">{{ viewVar.description }}</p>
                </div>
                <div class="grid grid-cols-2 gap-4">
                  <div>
                    <label
                      class="block text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1"
                      >{{ t('envVars.field.rec') }}</label
                    >
                    <code
                      v-if="viewVar.rec"
                      class="font-mono text-sm bg-sky-50 dark:bg-sky-900/30 text-sky-700 dark:text-sky-300 px-2 py-1 rounded break-all inline-block max-w-full"
                    >
                      {{ viewVar.rec }}
                    </code>
                    <span v-else class="text-gray-400 text-sm">—</span>
                  </div>
                  <div>
                    <label
                      class="block text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1"
                      >{{ t('envVars.field.def') }}</label
                    >
                    <code
                      v-if="viewVar.def"
                      class="font-mono text-sm bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 px-2 py-1 rounded break-all inline-block max-w-full"
                    >
                      {{ viewVar.def }}
                    </code>
                    <span v-else class="text-gray-400 text-sm">—</span>
                  </div>
                </div>
                <div>
                  <label
                    class="block text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1"
                    >{{ t('envVars.field.notes') }}</label
                  >
                  <p v-if="viewVar.notes" class="text-sm text-amber-600 dark:text-amber-400 flex items-start gap-1">
                    <iconify-icon icon="mdi:alert-circle-outline" width="14" class="shrink-0 mt-0.5"></iconify-icon>
                    {{ viewVar.notes }}
                  </p>
                </div>
                <div>
                  <label
                    class="block text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1"
                    >{{ t('envVars.status.set') }}</label
                  >
                  <p
                    v-if="viewVar.is_set"
                    class="font-mono text-sm text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-gray-900 rounded-lg px-3 py-2 border border-gray-100 dark:border-gray-700 break-all"
                  >
                    {{ viewVar.value }}
                  </p>
                  <p v-else class="text-sm text-gray-400 dark:text-gray-500 italic">
                    {{ t('envVars.status.notSet') }}
                  </p>
                </div>
              </div>

              <!-- Footer -->
              <div
                class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-100 dark:border-gray-700/50 flex items-center justify-end gap-2"
              >
                <button
                  class="h-9 px-4 rounded-lg text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  @click="viewVar = null"
                >
                  {{ t('common.action.cancel') }}
                </button>
              </div>
            </div>
          </Transition>
        </div>
      </Transition>
    </Teleport>

    <!-- Edit / Add modal -->
    <Teleport to="body">
      <Transition name="dialog-overlay">
        <div
          v-if="showEditModal"
          class="fixed inset-0 bg-black/40 backdrop-blur-[2px] flex items-center justify-center z-50"
          @click.self="closeEditModal"
        >
          <Transition name="dialog-panel" appear>
            <div
              class="bg-white dark:bg-gray-800 rounded-2xl w-full max-w-lg border border-gray-200 dark:border-gray-700 shadow-2xl"
            >
              <!-- Header -->
              <div class="px-6 pt-5 pb-4 border-b border-gray-100 dark:border-gray-700/50">
                <div class="flex items-center gap-3">
                  <div
                    class="w-9 h-9 rounded-xl bg-sky-50 dark:bg-sky-900/30 flex items-center justify-center shrink-0"
                  >
                    <iconify-icon icon="mdi:variable" width="20" class="text-sky-600 dark:text-sky-400"></iconify-icon>
                  </div>
                  <div>
                    <h2 class="text-base font-semibold text-gray-900 dark:text-gray-100">
                      {{ editingIdx !== null ? t('envVars.dialog.editTitle') : t('envVars.dialog.addTitle') }}
                    </h2>
                    <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                      {{ t('envVars.description') }}
                    </p>
                  </div>
                </div>
              </div>

              <!-- Body -->
              <div class="px-6 py-5 space-y-4">
                <!-- Category -->
                <div>
                  <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1.5">
                    {{ t('envVars.form.category') }}
                  </label>
                  <select
                    v-model="editForm.category"
                    class="w-full h-9 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg px-3 text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-sky-500"
                  >
                    <option v-for="cat in CATEGORY_KEYS" :key="cat" :value="cat">
                      {{ getCategoryLabel(cat) }}
                    </option>
                  </select>
                </div>

                <!-- Variable name -->
                <div>
                  <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1.5">
                    {{ t('envVars.form.variableName') }}
                  </label>
                  <input
                    v-model="editForm.name"
                    :placeholder="t('envVars.placeholder.nameInput')"
                    class="w-full h-9 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg px-3 text-sm font-mono text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500"
                  />
                </div>

                <!-- Description -->
                <div>
                  <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1.5">
                    {{ t('envVars.field.description') }}
                  </label>
                  <textarea
                    v-model="editForm.description"
                    :placeholder="t('envVars.placeholder.description')"
                    rows="2"
                    class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500 resize-none"
                  ></textarea>
                </div>

                <!-- Set value -->
                <div>
                  <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1.5">
                    {{ t('envVars.field.rec') }}
                  </label>
                  <input
                    v-model="editForm.rec"
                    class="w-full h-9 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg px-3 text-sm font-mono text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500"
                  />
                </div>

                <!-- Default value -->
                <div>
                  <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1.5">
                    {{ t('envVars.field.def') }}
                  </label>
                  <input
                    v-model="editForm.def"
                    class="w-full h-9 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg px-3 text-sm font-mono text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500"
                  />
                </div>

                <!-- Notes -->
                <div>
                  <label class="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1.5">
                    {{ t('envVars.field.notes') }}
                  </label>
                  <textarea
                    v-model="editForm.notes"
                    rows="2"
                    class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500 resize-none"
                  />
                </div>
              </div>

              <!-- Footer -->
              <div
                class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-100 dark:border-gray-700/50 flex items-center justify-end gap-2"
              >
                <button
                  class="h-9 px-4 rounded-lg text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  @click="closeEditModal"
                >
                  {{ t('common.action.cancel') }}
                </button>
                <button
                  class="h-9 px-4 rounded-lg text-sm font-medium bg-sky-600 hover:bg-sky-500 text-white shadow-sm transition-colors"
                  @click="saveEditModal"
                >
                  {{ t('common.action.save') }}
                </button>
              </div>
            </div>
          </Transition>
        </div>
      </Transition>
    </Teleport>

    <!-- Normal variable confirmation dialog -->
    <Teleport to="body">
      <Transition name="dialog-overlay">
        <div
          v-if="normalConfirm"
          class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/50"
          @click.self="normalConfirm = null"
        >
          <Transition name="dialog-panel">
            <div
              v-if="normalConfirm"
              class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl w-full max-w-sm mx-4 overflow-hidden"
            >
              <!-- Header -->
              <div class="px-6 pt-6 pb-4">
                <div class="flex items-center gap-3 mb-3">
                  <div
                    class="w-9 h-9 rounded-full bg-sky-100 dark:bg-sky-900/40 flex items-center justify-center shrink-0"
                  >
                    <iconify-icon
                      icon="mdi:information-outline"
                      width="20"
                      class="text-sky-600 dark:text-sky-400"
                    ></iconify-icon>
                  </div>
                  <h3 class="text-base font-bold text-gray-900 dark:text-gray-100">
                    {{ t('envVars.normalVar.title') }}
                  </h3>
                </div>

                <div class="bg-sky-50 dark:bg-sky-900/20 border border-sky-200 dark:border-sky-800 rounded-lg p-3">
                  <p class="text-sm text-sky-800 dark:text-sky-200">
                    {{
                      t('envVars.normalVar.' + (normalConfirm.type === 'apply' ? 'applyHint' : 'deactivateHint'), {
                        name: normalConfirm.variable.name,
                      })
                    }}
                  </p>
                </div>
              </div>

              <!-- Footer -->
              <div
                class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-100 dark:border-gray-700/50 flex items-center justify-end gap-2"
              >
                <button
                  class="h-9 px-4 rounded-lg text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  @click="normalConfirm = null"
                >
                  {{ t('common.action.cancel') }}
                </button>
                <button
                  class="h-9 px-4 rounded-lg text-sm font-medium bg-sky-600 hover:bg-sky-500 text-white shadow-sm transition-colors"
                  @click="handleNormalConfirm"
                >
                  {{ normalConfirm.type === 'apply' ? t('envVars.action.apply') : t('envVars.action.deactivate') }}
                </button>
              </div>
            </div>
          </Transition>
        </div>
      </Transition>
    </Teleport>

    <!-- Critical variable confirmation dialog -->
    <Teleport to="body">
      <Transition name="dialog-overlay">
        <div
          v-if="criticalConfirm"
          class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/50"
          @click.self="criticalConfirm = null"
        >
          <Transition name="dialog-panel">
            <div
              v-if="criticalConfirm"
              class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl w-full max-w-md mx-4 overflow-hidden"
            >
              <!-- Header -->
              <div class="px-6 pt-6 pb-4">
                <div class="flex items-center gap-3 mb-4">
                  <div
                    class="w-10 h-10 rounded-full bg-amber-100 dark:bg-amber-900/40 flex items-center justify-center shrink-0"
                  >
                    <iconify-icon
                      icon="mdi:alert-outline"
                      width="24"
                      class="text-amber-600 dark:text-amber-400"
                    ></iconify-icon>
                  </div>
                  <h3 class="text-lg font-bold text-gray-900 dark:text-gray-100">
                    {{ t('envVars.criticalVar.title') }}
                  </h3>
                </div>

                <!-- Warning content -->
                <div
                  class="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4 space-y-3"
                >
                  <p class="text-sm font-semibold text-amber-800 dark:text-amber-200">
                    {{
                      t(
                        'envVars.criticalVar.' +
                          (criticalConfirm.type === 'apply' ? 'applyWarning' : 'deactivateWarning'),
                        { name: criticalConfirm.variable.name }
                      )
                    }}
                  </p>

                  <ul class="space-y-1.5">
                    <template v-if="criticalConfirm.type === 'apply'">
                      <li class="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-300">
                        <iconify-icon icon="mdi:chevron-right" width="16" class="shrink-0 mt-0.5"></iconify-icon>
                        <span>{{ t('envVars.criticalVar.effect1', { name: criticalConfirm.variable.name }) }}</span>
                      </li>
                      <li class="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-300">
                        <iconify-icon icon="mdi:chevron-right" width="16" class="shrink-0 mt-0.5"></iconify-icon>
                        <span>{{ t('envVars.criticalVar.effect2', { name: criticalConfirm.variable.name }) }}</span>
                      </li>
                      <li class="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-300">
                        <iconify-icon icon="mdi:chevron-right" width="16" class="shrink-0 mt-0.5"></iconify-icon>
                        <span>{{ t('envVars.criticalVar.effect3', { name: criticalConfirm.variable.name }) }}</span>
                      </li>
                      <li class="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-300">
                        <iconify-icon icon="mdi:chevron-right" width="16" class="shrink-0 mt-0.5"></iconify-icon>
                        <span>{{ t('envVars.criticalVar.effect4', { name: criticalConfirm.variable.name }) }}</span>
                      </li>
                    </template>
                    <template v-else>
                      <li class="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-300">
                        <iconify-icon icon="mdi:chevron-right" width="16" class="shrink-0 mt-0.5"></iconify-icon>
                        <span>{{
                          t('envVars.criticalVar.deactivateEffect1', { name: criticalConfirm.variable.name })
                        }}</span>
                      </li>
                      <li class="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-300">
                        <iconify-icon icon="mdi:chevron-right" width="16" class="shrink-0 mt-0.5"></iconify-icon>
                        <span>{{
                          t('envVars.criticalVar.deactivateEffect2', { name: criticalConfirm.variable.name })
                        }}</span>
                      </li>
                      <li class="flex items-start gap-2 text-sm text-amber-700 dark:text-amber-300">
                        <iconify-icon icon="mdi:chevron-right" width="16" class="shrink-0 mt-0.5"></iconify-icon>
                        <span>{{
                          t('envVars.criticalVar.deactivateEffect3', { name: criticalConfirm.variable.name })
                        }}</span>
                      </li>
                    </template>
                  </ul>

                  <!-- PATH note -->
                  <div
                    class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md p-3 flex items-start gap-2"
                  >
                    <iconify-icon
                      icon="mdi:information-outline"
                      width="16"
                      class="text-blue-500 dark:text-blue-400 shrink-0 mt-0.5"
                    ></iconify-icon>
                    <p class="text-xs font-semibold text-blue-700 dark:text-blue-300">
                      {{ t('envVars.criticalVar.pathNote', { name: criticalConfirm.variable.name }) }}
                    </p>
                  </div>
                </div>
              </div>

              <!-- Confirm checkbox -->
              <div class="px-6 pb-4">
                <label class="flex items-center gap-2.5 cursor-pointer select-none">
                  <input
                    v-model="criticalConfirmed"
                    type="checkbox"
                    class="w-4 h-4 rounded border-gray-300 dark:border-gray-600 text-amber-600 focus:ring-amber-500"
                  />
                  <span class="text-sm text-gray-700 dark:text-gray-300 font-medium">
                    {{ t('envVars.criticalVar.confirm') }}
                  </span>
                </label>
              </div>

              <!-- Footer -->
              <div
                class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-100 dark:border-gray-700/50 flex items-center justify-end gap-2"
              >
                <button
                  class="h-9 px-4 rounded-lg text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  @click="criticalConfirm = null"
                >
                  {{ t('common.action.cancel') }}
                </button>
                <button
                  class="h-9 px-4 rounded-lg text-sm font-medium text-white shadow-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                  :class="
                    criticalConfirm.type === 'apply' ? 'bg-amber-600 hover:bg-amber-500' : 'bg-red-600 hover:bg-red-500'
                  "
                  :disabled="!criticalConfirmed"
                  @click="handleCriticalConfirm"
                >
                  {{ criticalConfirm.type === 'apply' ? t('envVars.action.apply') : t('envVars.action.deactivate') }}
                </button>
              </div>
            </div>
          </Transition>
        </div>
      </Transition>
    </Teleport>

    <!-- Delete confirmation dialog -->
    <ConfirmDialog
      v-if="confirmDelete"
      :title="t('envVars.dialog.deleteTitle')"
      :message="t('envVars.dialog.deleteConfirm', { name: confirmDelete.name })"
      :confirm-label="t('common.action.delete')"
      :danger="true"
      @confirm="deleteVar(confirmDelete)"
      @cancel="confirmDelete = null"
    />
  </div>
</template>

<style scoped>
/* Dialog overlay animation */
.dialog-overlay-enter-active,
.dialog-overlay-leave-active {
  transition: opacity 0.2s ease;
}
.dialog-overlay-enter-from,
.dialog-overlay-leave-to {
  opacity: 0;
}

/* Dialog panel animation */
.dialog-panel-enter-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.dialog-panel-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dialog-panel-enter-from {
  opacity: 0;
  transform: scale(0.95) translateY(8px);
}
.dialog-panel-leave-to {
  opacity: 0;
  transform: scale(0.95) translateY(8px);
}
</style>
