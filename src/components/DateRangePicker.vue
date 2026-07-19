<script setup lang="ts">
/**
 * DateRangePicker — A dropdown date range selector.
 *
 * Opens a single calendar in a dropdown. First click selects the start date,
 * second click selects the end date. The range between start and end is
 * highlighted. If end < start on second click, they are automatically swapped.
 *
 * Model: { start: string | null, end: string | null }  (YYYY-MM-DD format)
 */
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { useCalendar, fmtDate, WEEK_DAYS } from '../composables/useCalendar'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// ──────────────────────── Props / Emits ──────────────────────────

const props = withDefaults(
  defineProps<{
    /** Date range model: { start, end } as YYYY-MM-DD strings */
    modelValue?: { start: string | null; end: string | null } | null
    placeholder?: string
    disabled?: boolean
  }>(),
  {
    modelValue: () => ({ start: null, end: null }),
    placeholder: '',
    disabled: false,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: { start: string | null; end: string | null }]
}>()

// ──────────────────────── State ────────────────────────────────────

const open = ref(false)
const wrapperRef = ref<HTMLElement | null>(null)
/** Currently being picked; null = picking start, 'start' = start picked, picking end */
const pendingStart = ref<string | null>(null)

const cal = useCalendar()

// ──────────────────────── Derived ──────────────────────────────────

const displayText = computed(() => {
  const s = props.modelValue?.start
  const e = props.modelValue?.end
  if (s && e) return `${s} ~ ${e}`
  if (s) return `${s} ~ `
  return ''
})

/** Is a given date within the selected range? (exclusive of endpoints) */
function isInRange(dateStr: string): boolean {
  const s = props.modelValue?.start
  const e = props.modelValue?.end
  if (!s || !e) return false
  return dateStr > s && dateStr < e
}

/** Is a date the start or end? */
function isEndpoint(dateStr: string): 'start' | 'end' | null {
  if (dateStr === props.modelValue?.start) return 'start'
  if (dateStr === props.modelValue?.end) return 'end'
  return null
}

/** Compute cell classes for range visualization */
function cellClass(cell: { date: string; inMonth: boolean; isFuture: boolean }): string {
  const ep = isEndpoint(cell.date)
  const inRange = isInRange(cell.date)

  if (ep === 'start') {
    return 'bg-sky-500 text-white font-semibold rounded-l-full'
  }
  if (ep === 'end') {
    return 'bg-sky-500 text-white font-semibold rounded-r-full'
  }
  if (inRange) {
    return 'bg-sky-100 dark:bg-sky-900/40 text-sky-700 dark:text-sky-300 rounded-none'
  }
  if (cell.isFuture) {
    return 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
  }
  if (!cell.inMonth) {
    return 'text-gray-300 dark:text-gray-600'
  }
  // Normal clickable day
  return 'text-gray-700 dark:text-gray-300 hover:bg-sky-50 dark:hover:bg-sky-900/30'
}

// ──────────────────────── Actions ──────────────────────────────────

function toggleOpen() {
  if (props.disabled) return
  if (open.value) {
    open.value = false
    return
  }
  cal.initView(props.modelValue?.start ?? null)
  pendingStart.value = null
  open.value = true
}

function onClickCell(dateStr: string) {
  const s = props.modelValue?.start
  const e = props.modelValue?.end

  // Case 1: No start selected yet, or resetting after both selected
  if (!s || (s && e)) {
    emit('update:modelValue', { start: dateStr, end: null })
    pendingStart.value = dateStr
    return
  }

  // Case 2: Start selected, now picking end
  if (!e) {
    if (dateStr < s) {
      // User picked date before start → swap
      emit('update:modelValue', { start: dateStr, end: s })
    } else {
      emit('update:modelValue', { start: s, end: dateStr })
    }
    open.value = false
  }
}

function clear() {
  emit('update:modelValue', { start: null, end: null })
  pendingStart.value = null
  open.value = false
}

function selectToday() {
  const today = fmtDate(new Date().getFullYear(), new Date().getMonth() + 1, new Date().getDate())
  // Select today as start, leave end open
  emit('update:modelValue', { start: today, end: null })
  pendingStart.value = today
}

// ──────────────────────── Outside click ────────────────────────────

function onClickOutside(e: MouseEvent) {
  if (wrapperRef.value && !wrapperRef.value.contains(e.target as Node)) {
    open.value = false
  }
}

onMounted(() => document.addEventListener('mousedown', onClickOutside))
onBeforeUnmount(() => document.removeEventListener('mousedown', onClickOutside))

// ──────────────────────── Expose for testing ───────────────────────

defineExpose({ open, pendingStart, clear, onClickCell })
</script>

<template>
  <div ref="wrapperRef" class="relative">
    <!-- Input display -->
    <div
      class="h-9 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg flex items-center transition-shadow"
      :class="[
        open ? 'ring-2 ring-sky-500 border-transparent' : '',
        disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer',
      ]"
      @click="toggleOpen"
    >
      <iconify-icon icon="mdi:calendar-range" width="18" class="text-gray-400 ml-3 shrink-0" />
      <span
        v-if="displayText"
        class="w-full px-2 text-sm text-gray-900 dark:text-gray-100 truncate select-none"
      >{{ displayText }}</span>
      <span
        v-else
        class="w-full px-2 text-sm text-gray-400 select-none truncate"
      >{{ placeholder }}</span>
      <button
        v-if="modelValue?.start && !disabled"
        type="button"
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 mr-2 shrink-0 transition-colors"
        @click.stop="clear"
      >
        <iconify-icon icon="mdi:close-circle" width="16" />
      </button>
    </div>

    <!-- Dropdown Calendar -->
    <Transition
      enter-active-class="transition duration-150 ease-out"
      enter-from-class="opacity-0 scale-95 -translate-y-1"
      enter-to-class="opacity-100 scale-100 translate-y-0"
      leave-active-class="transition duration-100 ease-in"
      leave-from-class="opacity-100 scale-100"
      leave-to-class="opacity-0 scale-95 -translate-y-1"
    >
      <div
        v-if="open"
        class="absolute left-0 top-[calc(100%+4px)] z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl shadow-lg dark:shadow-gray-950/40 p-3 w-[280px]"
      >
        <!-- Header: nav + month/year -->
        <div class="flex items-center justify-between mb-2">
          <button
            type="button"
            class="w-7 h-7 flex items-center justify-center rounded-md text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            @click="cal.prevMonth()"
          >
            <iconify-icon icon="mdi:chevron-left" width="18" />
          </button>
          <span class="text-sm font-semibold text-gray-800 dark:text-gray-200 select-none">{{ cal.monthLabel.value }}</span>
          <button
            type="button"
            class="w-7 h-7 flex items-center justify-center rounded-md text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            @click="cal.nextMonth()"
          >
            <iconify-icon icon="mdi:chevron-right" width="18" />
          </button>
        </div>

        <!-- Weekday headers -->
        <div class="grid grid-cols-7 mb-1">
          <div
            v-for="d in WEEK_DAYS"
            :key="d"
            class="h-6 flex items-center justify-center text-[11px] font-medium text-gray-400 dark:text-gray-500 select-none"
          >
            {{ d }}
          </div>
        </div>

        <!-- Day grid -->
        <div class="grid grid-cols-7">
          <button
            v-for="cell in cal.calendarDays.value"
            :key="cell.date"
            type="button"
            class="h-8 flex items-center justify-center text-xs transition-colors"
            :class="[
              cellClass(cell),
              cell.isToday && isEndpoint(cell.date) === null && 'font-semibold text-sky-600 dark:text-sky-400',
            ]"
            :disabled="cell.isFuture"
            @click="!cell.isFuture && onClickCell(cell.date)"
          >
            {{ cell.day }}
          </button>
        </div>

        <!-- Footer: hint + actions -->
        <div class="flex items-center justify-between mt-2 pt-2 border-t border-gray-100 dark:border-gray-700">
          <button
            type="button"
            class="text-xs text-gray-500 dark:text-gray-400 hover:text-sky-600 dark:hover:text-sky-400 transition-colors"
            @click="selectToday"
          >
            {{ t('components.datePicker.today') }}
          </button>
          <span class="text-[11px] text-gray-400">
            {{ !modelValue?.start ? t('components.datePicker.selectStartDate') : !modelValue?.end ? t('components.datePicker.selectEndDate') : '' }}
          </span>
          <button
            v-if="modelValue?.start"
            type="button"
            class="text-xs text-gray-500 dark:text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition-colors"
            @click="clear"
          >
            {{ t('components.datePicker.clear') }}
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>