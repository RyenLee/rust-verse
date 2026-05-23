/**
 * Shared async utility: wrap a promise with a timeout.
 *
 * Usage:
 *   const result = await withTimeout(fetchData(), 5000)
 *   if (result.ok) { use result.value } else { handle timeout }
 */
export function withTimeout<T>(
  promise: Promise<T>,
  ms: number,
): Promise<{ ok: true; value: T } | { ok: false; reason: 'timeout' }> {
  return Promise.race([
    promise.then(value => ({ ok: true as const, value })),
    new Promise<{ ok: false; reason: 'timeout' }>(resolve =>
      setTimeout(() => resolve({ ok: false, reason: 'timeout' }), ms),
    ),
  ])
}