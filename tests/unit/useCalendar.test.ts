/**
 * Unit tests for useCalendar composable and date utilities.
 */
import { describe, test, expect, beforeEach } from 'vitest'
import { todayStr, fmtDate, parseDate, generateCalendarGrid, useCalendar } from '../../src/composables/useCalendar'

// ───────────────────────────────────────────────────────────────────
// fmtDate
// ───────────────────────────────────────────────────────────────────
describe('fmtDate', () => {
  test('formats YYYY-MM-DD with padding', () => {
    expect(fmtDate(2025, 1, 1)).toBe('2025-01-01')
    expect(fmtDate(2025, 12, 31)).toBe('2025-12-31')
    expect(fmtDate(2025, 5, 5)).toBe('2025-05-05')
  })

  test('pads single-digit month and day', () => {
    expect(fmtDate(2024, 3, 9)).toBe('2024-03-09')
    expect(fmtDate(2024, 11, 7)).toBe('2024-11-07')
  })
})

// ───────────────────────────────────────────────────────────────────
// parseDate
// ───────────────────────────────────────────────────────────────────
describe('parseDate', () => {
  test('parses valid date string', () => {
    const result = parseDate('2025-06-15')
    expect(result).toEqual({ year: 2025, month: 6, day: 15 })
  })

  test('returns null for null input', () => {
    expect(parseDate(null)).toBeNull()
  })

  test('returns null for empty string', () => {
    expect(parseDate('')).toBeNull()
  })

  test('returns null for malformed date', () => {
    expect(parseDate('not-a-date')).toBeNull()
    expect(parseDate('')).toBeNull()
    expect(parseDate('abc-def-ghi')).toBeNull()
  })
})

// ───────────────────────────────────────────────────────────────────
// todayStr
// ───────────────────────────────────────────────────────────────────
describe('todayStr', () => {
  test('returns YYYY-MM-DD format', () => {
    const result = todayStr()
    expect(result).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  test('matches actual today', () => {
    const now = new Date()
    const expected = fmtDate(now.getFullYear(), now.getMonth() + 1, now.getDate())
    expect(todayStr()).toBe(expected)
  })
})

// ───────────────────────────────────────────────────────────────────
// generateCalendarGrid
// ───────────────────────────────────────────────────────────────────
describe('generateCalendarGrid', () => {
  test('returns 42 cells (6 rows)', () => {
    const grid = generateCalendarGrid(2025, 5, '2025-05-25')
    expect(grid.length).toBe(42)
  })

  test('first cell of month starting on Wednesday (2025-01-01)', () => {
    // Jan 2025 starts on Wednesday → offset 2 (Mon-start), so 2 prev-month cells
    const grid = generateCalendarGrid(2025, 1, '2025-06-01')
    // First 2 should be prev month (Dec 2024 Mon/Tue), then Jan 1
    expect(grid[0].inMonth).toBe(false)
    expect(grid[0].date).toBe('2024-12-30')
    expect(grid[1].inMonth).toBe(false)
    expect(grid[1].date).toBe('2024-12-31')
    expect(grid[2].inMonth).toBe(true)
    expect(grid[2].day).toBe(1)
    expect(grid[2].date).toBe('2025-01-01')
  })

  test('marks today correctly', () => {
    const today = '2025-05-25'
    const grid = generateCalendarGrid(2025, 5, today)
    const todayCell = grid.find(c => c.isToday)
    expect(todayCell).toBeDefined()
    expect(todayCell!.date).toBe(today)
    expect(todayCell!.inMonth).toBe(true)
    expect(todayCell!.day).toBe(25)
  })

  test('marks future dates as disabled', () => {
    const today = '2025-05-15'
    const grid = generateCalendarGrid(2025, 5, today)
    const futureCells = grid.filter(c => c.isFuture)
    expect(futureCells.length).toBeGreaterThan(0)
    futureCells.forEach(c => {
      expect(c.date > today).toBe(true)
    })
  })

  test('no future dates marked when today is end of year', () => {
    const today = '2025-12-31'
    const grid = generateCalendarGrid(2025, 12, today)
    // Dec grid may have some next-month padding in January, those should be future
    const nextYearCells = grid.filter(c => c.date.startsWith('2026'))
    nextYearCells.forEach(c => {
      expect(c.isFuture).toBe(true)
    })
  })

  test('has correct number of in-month days', () => {
    // May has 31 days
    const grid = generateCalendarGrid(2025, 5, '2025-05-25')
    const inMonth = grid.filter(c => c.inMonth)
    expect(inMonth.length).toBe(31)
  })

  test('February handles leap year (2024)', () => {
    const grid = generateCalendarGrid(2024, 2, '2024-06-01')
    const inMonth = grid.filter(c => c.inMonth)
    expect(inMonth.length).toBe(29)
  })

  test('February handles non-leap year (2025)', () => {
    const grid = generateCalendarGrid(2025, 2, '2025-06-01')
    const inMonth = grid.filter(c => c.inMonth)
    expect(inMonth.length).toBe(28)
  })

  test('date strings are properly formatted', () => {
    const grid = generateCalendarGrid(2025, 5, '2025-05-25')
    grid.forEach(c => {
      expect(c.date).toMatch(/^\d{4}-\d{2}-\d{2}$/)
    })
  })
})

// ───────────────────────────────────────────────────────────────────
// useCalendar — month navigation
// ───────────────────────────────────────────────────────────────────
describe('useCalendar', () => {
  test('initView sets to given date', () => {
    const cal = useCalendar()
    cal.initView('2025-06-15')
    expect(cal.viewYear.value).toBe(2025)
    expect(cal.viewMonth.value).toBe(6)
    expect(cal.monthLabel.value).toBe('2025年6月')
  })

  test('initView without date sets to current month', () => {
    const cal = useCalendar()
    cal.initView(null)
    const now = new Date()
    expect(cal.viewYear.value).toBe(now.getFullYear())
    expect(cal.viewMonth.value).toBe(now.getMonth() + 1)
  })

  test('nextMonth wraps year correctly', () => {
    const cal = useCalendar()
    cal.initView('2025-12-01')
    expect(cal.viewMonth.value).toBe(12)
    cal.nextMonth()
    expect(cal.viewYear.value).toBe(2026)
    expect(cal.viewMonth.value).toBe(1)
  })

  test('prevMonth wraps year correctly', () => {
    const cal = useCalendar()
    cal.initView('2025-01-15')
    expect(cal.viewMonth.value).toBe(1)
    cal.prevMonth()
    expect(cal.viewYear.value).toBe(2024)
    expect(cal.viewMonth.value).toBe(12)
  })

  test('nextMonth advances within year', () => {
    const cal = useCalendar()
    cal.initView('2025-06-01')
    cal.nextMonth()
    expect(cal.viewYear.value).toBe(2025)
    expect(cal.viewMonth.value).toBe(7)
  })

  test('prevMonth goes back within year', () => {
    const cal = useCalendar()
    cal.initView('2025-06-01')
    cal.prevMonth()
    expect(cal.viewYear.value).toBe(2025)
    expect(cal.viewMonth.value).toBe(5)
  })

  test('calendarDays recomputes on month change', () => {
    const cal = useCalendar()
    cal.initView('2025-05-15')
    const mayDays = cal.calendarDays.value.filter(c => c.inMonth).length
    expect(mayDays).toBe(31)
    cal.nextMonth() // June
    const juneDays = cal.calendarDays.value.filter(c => c.inMonth).length
    expect(juneDays).toBe(30)
  })
})
