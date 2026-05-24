<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, inject, type Ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '../composables/useAppStore'
import { setLocale, getLocale, getAvailableLocales, type LocaleInfo } from '../locales'
import HelpPanel from './HelpPanel.vue'

defineEmits<{
  'toggle-sidebar': []
}>()

const { t } = useI18n()
const { isDark, toggleTheme } = useAppStore()
const router = useRouter()

// Global update notification (injected from App.vue)
const updateAvailableInfo = inject<Ref<{ version: string; currentVersion: string } | null>>('updateAvailableInfo')
const dismissUpdateNotification = inject<() => void>('dismissUpdateNotification')

const currentLocale = ref(getLocale())
const availableLocales = ref<LocaleInfo[]>(getAvailableLocales())
const langOpen = ref(false)
const langDropdownRef = ref<HTMLElement | null>(null)
const showHelp = ref(false)

const hasUpdate = computed(() => !!updateAvailableInfo?.value)

const currentLocaleInfo = computed(() =>
  availableLocales.value.find(l => l.code === currentLocale.value)
)

function selectLocale(code: string) {
  currentLocale.value = code
  setLocale(code)
  langOpen.value = false
}

function handleUpdateClick() {
  if (dismissUpdateNotification) {
    dismissUpdateNotification()
  }
  router.push('/about')
}

function handleClickOutside(e: MouseEvent) {
  if (langDropdownRef.value && !langDropdownRef.value.contains(e.target as Node)) {
    langOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside, true)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside, true)
})
</script>

<template>
  <header class="shrink-0 h-14 border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-950 flex items-center px-4 gap-3">
    <!-- Left: sidebar toggle -->
    <button
      class="p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
      title="Toggle sidebar (Ctrl+B)"
      @click="$emit('toggle-sidebar')"
    >
      <iconify-icon icon="mdi:menu" width="20"></iconify-icon>
    </button>

    <!-- Divider -->
    <div class="w-px h-6 bg-gray-200 dark:bg-gray-700"></div>

    <!-- Center: slot for breadcrumb/title (filled by PageLayout) -->
    <div class="flex-1"></div>

    <!-- Right: update badge, help, theme, language -->
    <div class="flex items-center gap-1">
      <!-- Update available badge -->
      <button
        v-if="hasUpdate"
        class="relative p-2 rounded-lg text-orange-500 hover:text-orange-600 hover:bg-orange-50 dark:hover:bg-orange-900/20 transition-colors"
        title="New version available — click to update"
        @click="handleUpdateClick"
      >
        <iconify-icon icon="mdi:update" width="20"></iconify-icon>
        <!-- Red dot badge -->
        <span class="absolute top-1.5 right-1.5 w-2 h-2 bg-red-500 rounded-full animate-pulse"></span>
      </button>

      <!-- Help button -->
      <button
        class="p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
        :title="t('nav.help')"
        @click="showHelp = true"
      >
        <iconify-icon icon="mdi:help-circle-outline" width="20"></iconify-icon>
      </button>

      <!-- Theme toggle -->
      <button
        class="p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
        :title="isDark ? t('app.lightMode') : t('app.darkMode')"
        @click="toggleTheme"
      >
        <iconify-icon
          :icon="isDark ? 'mdi:weather-sunny' : 'mdi:weather-night'"
          width="20"
          :class="isDark ? 'text-amber-400' : 'text-indigo-400'"
        ></iconify-icon>
      </button>

      <!-- Language switcher -->
      <div class="relative" ref="langDropdownRef">
        <button
          class="p-2 rounded-lg text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
          :title="currentLocaleInfo?.name"
          @click="langOpen = !langOpen"
        >
          <iconify-icon icon="mdi:translate" width="20"></iconify-icon>
        </button>
        <!-- Dropdown -->
        <Transition name="dropdown">
          <div
            v-if="langOpen"
            class="absolute right-0 top-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg overflow-hidden z-50 min-w-[160px]"
          >
            <button
              v-for="loc in availableLocales"
              :key="loc.code"
              class="flex items-center gap-2 w-full px-3 py-2 text-sm transition-colors"
              :class="loc.code === currentLocale ? 'bg-sky-50 dark:bg-sky-950/40 text-sky-700 dark:text-sky-300' : 'text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-700/50'"
              @click="selectLocale(loc.code)"
            >
              <iconify-icon
                icon="mdi:check"
                width="16"
                :class="loc.code === currentLocale ? 'text-sky-600 dark:text-sky-400' : 'text-transparent'"
              ></iconify-icon>
              <span class="flex-1 text-left">{{ loc.name }}</span>
            </button>
          </div>
        </Transition>
      </div>
    </div>

    <!-- Help panel -->
    <HelpPanel :visible="showHelp" @close="showHelp = false" />
  </header>
</template>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
