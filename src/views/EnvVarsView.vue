<script setup lang="ts">
import { onMounted, ref, computed, reactive } from 'vue'
import BaseButton from '../components/BaseButton.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { useEnvVars, type EnvVarInfo } from '../composables/useEnvVars'
import { usePersist } from '../composables/usePersist'
import { useToast } from '../composables/useToast'
import { useDataRefresh } from '../composables/useDataRefresh'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const { listEnvVars, setEnvVar, removeEnvVar, updateEnvVarMeta, deleteEnvVarMeta } = useEnvVars()
const { persistEnvVar, removePersistedEnvVar, listPersistedEnvVars } = usePersist()
const { success, error } = useToast()
const { notifyEnvVarChange } = useDataRefresh()

const envVars = ref<EnvVarInfo[]>([])
const loading = ref(true)
const searchQuery = ref('')
const activeCategory = ref('all')

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
    console.error('Failed to load env vars:', e)
    error(t('envVars.message.loadFailed'))
  } finally {
    loading.value = false
  }
}

async function loadPersistedStatus() {
  try {
    const list = await listPersistedEnvVars()
    persistedVars.value = new Set(list)
  } catch (e) {
    console.error('Failed to load persisted status:', e)
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
  try {
    // 1. Set in current process
    await setEnvVar(v.name, v.rec)
    // 2. Persist to system
    await persistEnvVar(v.name, v.rec)
    persistedVars.value.add(v.name)
    persistedVars.value = new Set(persistedVars.value)
    success(t('envVars.message.applySuccess', { name: v.name, value: v.rec }))
    notifyEnvVarChange()
    await loadData()
  } catch (e: any) {
    error(t('envVars.message.applyFailed', { error: String(e) }))
  }
}

// Deactivate: remove from system persistence AND unset from current process
async function deactivateVar(v: EnvVarInfo) {
  try {
    // 1. Remove from system persistence (registry / shell config)
    await removePersistedEnvVar(v.name)
    // 2. Remove from current process
    await removeEnvVar(v.name)
    // 3. Update local tracking
    persistedVars.value.delete(v.name)
    persistedVars.value = new Set(persistedVars.value)
    success(t('envVars.message.deactivateSuccess', { name: v.name }))
    notifyEnvVarChange()
    await loadData()
  } catch (e: any) {
    console.error('Failed to deactivate env var:', v.name, e)
    error(t('envVars.message.deactivateFailed', { error: String(e) }))
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
    <!-- Fixed header area -->
    <div class="shrink-0 px-6 lg:px-8 pt-6 lg:pt-8 pb-4 w-full space-y-4">
      <!-- Title row -->
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('envVars.title') }}</h1>
          <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">{{ t('envVars.description') }}</p>
        </div>
        <div class="flex items-center gap-2">
          <BaseButton variant="secondary" :loading="loading" @click="loadData">
            <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
            {{ t('common.action.refresh') }}
          </BaseButton>
          <BaseButton @click="openAddModal">
            <iconify-icon icon="mdi:plus" width="16"></iconify-icon>
            {{ t('envVars.action.addVariable') }}
          </BaseButton>
        </div>
      </div>

      <!-- Search -->
      <div class="relative min-w-0">
        <iconify-icon
          icon="mdi:magnify"
          width="18"
          class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
        ></iconify-icon>
        <input
          v-model="searchQuery"
          :placeholder="t('envVars.placeholder.search')"
          class="w-full h-9 box-border pl-10 pr-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:border-transparent"
        />
      </div>

      <!-- Category tabs -->
      <div class="flex gap-1 overflow-x-auto pb-1">
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

      <!-- Warning: Apply will modify system env vars -->
      <div
        class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg px-3 py-2.5 flex items-start gap-2"
      >
        <iconify-icon
          icon="mdi:alert-octagon-outline"
          width="16"
          class="text-red-500 dark:text-red-400 mt-0.5 shrink-0"
        ></iconify-icon>
        <p class="text-sm text-red-700 dark:text-red-300">{{ t('envVars.warning') }}</p>
      </div>
    </div>

    <!-- Scrollable content area -->
    <div class="flex-1 overflow-y-auto px-6 lg:px-8 pb-6 lg:pb-8">
      <div class="max-w-6xl mx-auto">
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

        <!-- Table layout -->
        <div v-else>
          <div v-if="filteredVars.length === 0" class="flex flex-col items-center justify-center py-16 text-center">
            <div class="w-14 h-14 rounded-2xl bg-gray-50 dark:bg-gray-800 flex items-center justify-center mb-3">
              <iconify-icon icon="mdi:variable" width="28" class="text-gray-400"></iconify-icon>
            </div>
            <p class="text-gray-500 dark:text-gray-400 text-sm">
              {{ searchQuery ? t('envVars.status.noMatching') : t('envVars.status.noVars') }}
            </p>
          </div>

          <div v-else class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
            <div class="overflow-y-auto max-h-full">
            <table class="w-full text-sm table-fixed">
              <thead class="sticky top-0 z-10">
                <tr class="bg-gray-50 dark:bg-gray-800/80">
                  <th class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap max-w-0">
                    {{ t('envVars.field.variable') }}
                  </th>
                  <th class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {{ t('envVars.field.description') }}
                  </th>
                  <th class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {{ t('envVars.field.rec') }}
                  </th>
                  <th class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {{ t('envVars.field.def') }}
                  </th>
                  <th class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {{ t('envVars.field.notes') }}
                  </th>
                  <th
                    class="text-center px-3 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap w-[160px]"
                  >
                    {{ t('envVars.field.actions') }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(v, idx) in filteredVars"
                  :key="v.name"
                  class="border-t border-gray-100 dark:border-gray-700/50 hover:bg-gray-50/50 dark:hover:bg-gray-800/50 transition-colors"
                >
                  <!-- Variable name -->
                  <td class="px-4 py-3 max-w-0">
                    <div class="flex items-center gap-2 min-w-0">
                      <span
                        class="shrink-0 w-2 h-2 rounded-full"
                        :class="v.is_set ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
                        :title="v.is_set ? t('envVars.status.set') : t('envVars.status.notSet')"
                      />
                      <span class="font-mono text-sm font-semibold text-gray-900 dark:text-gray-100 truncate" :title="v.name">
                        {{ v.name }}
                      </span>
                      <span
                        v-if="persistedVars.has(v.name)"
                        class="text-xs px-1.5 py-0.5 rounded bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300 flex items-center gap-0.5 shrink-0"
                        :title="t('envVars.persist.label')"
                      >
                        <iconify-icon icon="mdi:pin" width="11"></iconify-icon>
                        {{ t('envVars.persist.label') }}
                      </span>
                    </div>
                  </td>

                  <!-- Description -->
                  <td class="px-4 py-3 text-gray-600 dark:text-gray-400">
                    <span class="line-clamp-2">{{ v.description }}</span>
                  </td>

                  <!-- Recommended value -->
                  <td class="px-4 py-3">
                    <code
                      v-if="v.rec"
                      class="font-mono text-xs bg-sky-50 dark:bg-sky-900/30 text-sky-700 dark:text-sky-300 px-2 py-1 rounded truncate block"
                    >
                      {{ v.rec }}
                    </code>
                    <span v-else class="text-gray-300 dark:text-gray-600">—</span>
                  </td>

                  <!-- Default value -->
                  <td class="px-4 py-3">
                    <span v-if="v.def" class="font-mono text-xs text-gray-500 dark:text-gray-400 truncate block">
                      {{ v.def }}
                    </span>
                    <span v-else class="text-gray-300 dark:text-gray-600">—</span>
                  </td>

                  <!-- Notes -->
                  <td class="px-4 py-3">
                    <span
                      v-if="v.notes"
                      class="text-xs text-amber-600 dark:text-amber-400 line-clamp-2"
                      :title="v.notes"
                    >
                      {{ v.notes }}
                    </span>
                  </td>

                  <!-- Actions -->
                  <td class="px-2 py-3 w-[160px]">
                    <div class="flex items-center justify-center gap-1 flex-wrap max-h-[40px] overflow-hidden">
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
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
            </div>
          </div>
        </div>
      </div>
    </div>

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
                  <p class="text-sm text-gray-700 dark:text-gray-300">{{ viewVar.description }}</p>
                </div>
                <div class="grid grid-cols-2 gap-4">
                  <div>
                    <label
                      class="block text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-1"
                      >{{ t('envVars.field.rec') }}</label
                    >
                    <code
                      v-if="viewVar.rec"
                      class="font-mono text-sm bg-sky-50 dark:bg-sky-900/30 text-sky-700 dark:text-sky-300 px-2 py-1 rounded"
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
                      class="font-mono text-sm bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 px-2 py-1 rounded"
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
                  <input
                    v-model="editForm.description"
                    :placeholder="t('envVars.placeholder.description')"
                    class="w-full h-9 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg px-3 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500"
                  />
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
