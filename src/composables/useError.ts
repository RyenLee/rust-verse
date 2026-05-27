/**
 * Extract a human-readable error message from a Tauri command error.
 * Tauri serializes AppError as { kind: string, message: string }.
 * The invoke wrapper may throw the raw object, a string, or a standard Error.
 */
export function extractErrorMessage(e: unknown): string {
  if (typeof e === 'string') return e
  if (e instanceof Error) return e.message
  if (e && typeof e === 'object') {
    // Tauri AppError shape: { kind, message }
    const obj = e as Record<string, unknown>
    if (typeof obj.message === 'string') return obj.message
    // Fallback: try to get any string representation
    try { return JSON.stringify(e) } catch { /* ignore */ }
  }
  return String(e)
}