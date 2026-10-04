/** Entire receive sizing pipeline, including server and wallet validation, is finite (#1376). */
export const RECEIVE_QUOTE_TIMEOUT_MS = 70_000

export async function withReceiveQuoteDeadline<T>(
  signal: AbortSignal | undefined,
  work: (signal: AbortSignal) => Promise<T>
): Promise<T> {
  const controller = new AbortController()
  const timer = setTimeout(
    () => controller.abort(new Error('Receive quote timed out. Retry the quote.')),
    RECEIVE_QUOTE_TIMEOUT_MS
  )
  const combined = signal ? AbortSignal.any([signal, controller.signal]) : controller.signal
  let onAbort: (() => void) | undefined
  try {
    combined.throwIfAborted()
    return await Promise.race([
      work(combined),
      new Promise<never>((_resolve, reject) => {
        onAbort = () => reject(combined.reason)
        combined.addEventListener('abort', onAbort, { once: true })
        if (combined.aborted) onAbort()
      }),
    ])
  } finally {
    clearTimeout(timer)
    if (onAbort) combined.removeEventListener('abort', onAbort)
  }
}
