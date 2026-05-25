<script setup lang="ts">
/**
 * DatePicker — Single date selection with dropdown calendar.
 * Uses the shared useCalendar composable for calendar grid generation.
 */
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useCalendar, fmtDate, parseDate, WEEK_DAYS } from '../composables/useCalendar'

// ──────────────────────── Props / Emits ──────────────────────────

const props = withDefaults(
  defineProps<{
    modelValue?: string | null
    placeholder?: string
    disabled?: boolean
  }>(),
  {
    modelValue: null,
    placeholder: '',
    disabled: false,
  }
)

const emit = defineEmits<{
  'update:modelValue': [value: string | null]
}>()

// ──────────────────────── State ──────────────────────────────────

const open = ref(false)
const wrapperRef = ref<HTMLElement | null>(null)
const inputRef = ref<HTMLInputElement | null>(null)
const cal = useCalendar()

// ──────────────────────── Derived ─────────────────────────────────

const selectedDate = computed(() => parseDate(props.modelValue))

// ──────────────────────── Actions ────────────────────────────────

function toggleOpen() {
  if (props.disabled) return
  if (open.value) {
    open.value = false
    return
  }
  cal.initView(props.modelValue)
  open.value = true
}

function selectDay(date: string) {
  emit('update:modelValue', date)
  open.value = false
}

function clear() {
  emit('update:modelValue', null)
  open.value = false
}

function onInputKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    const val = (e.target as HTMLInputElement).value
    if (/^\d{4}-\d{2}-\d{2}$/.test(val)) {
      emit('update:modelValue', val)
      open.value = false
    }
  }
}

function onInputChange(e: Event) {
  const val = (e.target as HTMLInputElement).value
  if (/^\d{4}-\d{2}-\d{2}$/.test(val)) {
    emit('update:modelValue', val)
  }
}

// ──────────────────────── Outside click ──────────────────────────

function onClickOutside(e: MouseEvent) {
  if (wrapperRef.value && !wrapperRef.value.contains(e.target as Node)) {
    open.value = false
  }
}

onMounted(() => document.addEventListener('mousedown', onClickOutside))
onBeforeUnmount(() => document.removeEventListener('mousedown', onClickOutside))
</script>

<template>
  <div ref="wrapperRef" class="relative">
    <!-- Input -->
    <div
      class="h-9 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg flex items-center transition-shadow"
      :class="[
        open ? 'ring-2 ring-sky-500 border-transparent' : '',
        disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer',
      ]"
      @click="toggleOpen"
    >
      <iconify-icon icon="mdi:calendar" width="18" class="text-gray-400 ml-3 shrink-0" />
      <input
        ref="inputRef"
        type="text"
        :value="modelValue"
        :placeholder="placeholder"
        :disabled="disabled"
        class="w-full h-full px-2 bg-transparent text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none select-auto"
        @keydown="onInputKeydown"
        @change="onInputChange"
      />
      <button
        v-if="modelValue && !disabled"
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
        class="absolute left-0 top-[calc(100%+4px)] z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl shadow-lg dark:shadow-gray-950/40 p-3 w-[260px]"
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
          <span class="text-sm font-semibold text-gray-800 dark:text-gray-200 select-none">{{
            cal.monthLabel.value
          }}</span>
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
            class="h-8 flex items-center justify-center text-xs rounded-md transition-colors"
            :class="[
              cell.isFuture
                ? 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
                : cell.inMonth
                ? 'text-gray-700 dark:text-gray-300'
                : 'text-gray-300 dark:text-gray-600',
              cell.inMonth &&
                cell.date !== modelValue &&
                !cell.isFuture &&
                'hover:bg-sky-50 dark:hover:bg-sky-900/30 hover:text-sky-700 dark:hover:text-sky-300',
              cell.date === modelValue && 'bg-sky-500 text-white font-semibold',
              cell.isToday && cell.date !== modelValue && 'font-semibold text-sky-600 dark:text-sky-400',
            ]"
            :disabled="cell.isFuture"
            @click="selectDay(cell.date)"
          >
            {{ cell.day }}
          </button>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-between mt-2 pt-2 border-t border-gray-100 dark:border-gray-700">
          <button
            type="button"
            class="text-xs text-gray-500 dark:text-gray-400 hover:text-sky-600 dark:hover:text-sky-400 transition-colors"
            @click="selectDay(fmtDate(new Date().getFullYear(), new Date().getMonth() + 1, new Date().getDate()))"
          >
            今天
          </button>
          <button
            v-if="modelValue"
            type="button"
            class="text-xs text-gray-500 dark:text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition-colors"
            @click="clear"
          >
            清除
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>
