/**
 * useCalendar — Pure calendar grid generation logic.
 * Shared by DatePicker and DateRangePicker.
 */
import { computed, ref } from 'vue'

/** Format a date as YYYY-MM-DD string */
export function fmtDate(y: number, m: number, d: number): string {
  return `${y}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`
}

/** Parse YYYY-MM-DD string into { year, month, day } */
export function parseDate(dateStr: string | null): { year: number; month: number; day: number } | null {
  if (!dateStr) return null
  const [y, m, d] = dateStr.split('-').map(Number)
  if (!y || !m || !d) return null
  return { year: y, month: m, day: d }
}

/** Today as YYYY-MM-DD string */
export function todayStr(): string {
  const now = new Date()
  return fmtDate(now.getFullYear(), now.getMonth() + 1, now.getDate())
}

/** Week day labels (Monday-start) */
export const WEEK_DAYS = ['一', '二', '三', '四', '五', '六', '日']

export interface CalendarDay {
  date: string       // YYYY-MM-DD
  day: number        // day of month (1-31)
  inMonth: boolean   // belongs to current viewing month
  isToday: boolean
  isFuture: boolean  // after today → disabled
}

/**
 * Generate a 42-cell (6-row) calendar grid for the given view year/month.
 * @param year   - view year
 * @param month  - view month (1-12)
 * @param today  - today string for marking + disabling future dates
 */
export function generateCalendarGrid(
  year: number,
  month: number,
  today: string,
): CalendarDay[] {
  const firstDay = new Date(year, month - 1, 1).getDay()
  const startOffset = firstDay === 0 ? 6 : firstDay - 1
  const daysInMonth = new Date(year, month, 0).getDate()
  const prevDays = new Date(year, month - 1, 0).getDate()

  const days: CalendarDay[] = []

  // Previous month padding
  for (let i = startOffset - 1; i >= 0; i--) {
    const day = prevDays - i
    const pm = month === 1 ? 12 : month - 1
    const py = month === 1 ? year - 1 : year
    const dateStr = fmtDate(py, pm, day)
    days.push({ date: dateStr, day, inMonth: false, isToday: false, isFuture: dateStr > today })
  }

  // Current month
  for (let d = 1; d <= daysInMonth; d++) {
    const dateStr = fmtDate(year, month, d)
    days.push({
      date: dateStr,
      day: d,
      inMonth: true,
      isToday: dateStr === today,
      isFuture: dateStr > today,
    })
  }

  // Next month padding (fill to 42 cells)
  const remaining = 42 - days.length
  for (let i = 1; i <= remaining; i++) {
    const nm = month === 12 ? 1 : month + 1
    const ny = month === 12 ? year + 1 : year
    const dateStr = fmtDate(ny, nm, i)
    days.push({ date: dateStr, day: i, inMonth: false, isToday: false, isFuture: dateStr > today })
  }

  return days
}

/** Composable providing reactive calendar state and navigation */
export function useCalendar(initialDate?: string | null) {
  const viewYear = ref(0)
  const viewMonth = ref(0)

  const today = todayStr()
  const parsed = parseDate(initialDate ?? null)

  /** Initialize or re-center view to a given date (or today) */
  function initView(date: string | null) {
    const parsedDate = parseDate(date)
    if (parsedDate) {
      viewYear.value = parsedDate.year
      viewMonth.value = parsedDate.month
    } else {
      const now = new Date()
      viewYear.value = now.getFullYear()
      viewMonth.value = now.getMonth() + 1
    }
  }

  const calendarDays = computed(() =>
    generateCalendarGrid(viewYear.value, viewMonth.value, today),
  )

  const monthLabel = computed(() => `${viewYear.value}年${viewMonth.value}月`)

  function prevMonth() {
    if (viewMonth.value === 1) {
      viewMonth.value = 12
      viewYear.value--
    } else {
      viewMonth.value--
    }
  }

  function nextMonth() {
    if (viewMonth.value === 12) {
      viewMonth.value = 1
      viewYear.value++
    } else {
      viewMonth.value++
    }
  }

  return { viewYear, viewMonth, calendarDays, monthLabel, today, initView, prevMonth, nextMonth }
}