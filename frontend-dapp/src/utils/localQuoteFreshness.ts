import { SIM_QUOTE_REFETCH_INTERVAL_MS, simQuoteRefetchInterval } from '@/utils/quoteDebounce'

/** Original observation expiry, including the nested quote used by receive sizing.
 * Undefined means a legacy quote; NaN means malformed local metadata and fails closed. */
export function localQuoteExpiryMs(data: unknown): number | undefined {
  if (!data || typeof data !== 'object') return undefined
  const value = data as { executionMetadata?: unknown; quote?: unknown }
  const quoted = value.quote ?? value
  if (!quoted || typeof quoted !== 'object') return undefined
  const metadata = (quoted as { executionMetadata?: unknown }).executionMetadata
  if (metadata == null) return undefined
  if (typeof metadata !== 'object') return NaN
  const expiry = (metadata as { expires_at_ms?: unknown }).expires_at_ms
  return typeof expiry === 'number' && Number.isSafeInteger(expiry) && expiry > 0 ? expiry : NaN
}

export function isExpiredLocalQuote(data: unknown, nowMs = Date.now()): boolean {
  const expiry = localQuoteExpiryMs(data)
  return expiry !== undefined && (!Number.isFinite(expiry) || expiry <= nowMs)
}

/** React Query freshness cannot outlive the original execution snapshot. */
export function localQuoteStaleTime(query: { state: { data?: unknown; dataUpdatedAt: number } }): number {
  const expiry = localQuoteExpiryMs(query.state.data)
  if (expiry === undefined) return SIM_QUOTE_REFETCH_INTERVAL_MS
  if (!Number.isFinite(expiry)) return 0
  return Math.max(0, Math.min(SIM_QUOTE_REFETCH_INTERVAL_MS, expiry - query.state.dataUpdatedAt))
}

/** Reserve a bounded response window before the original display/signing deadline.
 * This does not change staleTime, expiry or failed-refresh cadence. */
export const LOCAL_QUOTE_REFRESH_HEADROOM_MS = 1_000

/** Refresh local arithmetic before expiry without polling producers or extending
 * the quote. A one-second minimum bounds repeated near-expiry cache responses. */
export function localQuoteRefetchInterval(query: {
  state: { data?: unknown; dataUpdatedAt?: number; fetchStatus: 'fetching' | 'paused' | 'idle'; error?: unknown }
}): number | false {
  const interval = simQuoteRefetchInterval(query)
  // Failed refreshes retain old data; expiry must not accelerate their retry cadence.
  if (interval === false || query.state.error != null) return interval
  const expiry = localQuoteExpiryMs(query.state.data)
  if (expiry === undefined) return interval
  if (!Number.isFinite(expiry)) return false
  // React Query re-evaluates interval callbacks on observer renders. A moving
  // Date.now() duration resets its timer repeatedly and can postpone a10s
  // refresh until expiry. The response publication anchor is stable through
  // unrelated renders; it does not change the original observation deadline.
  const anchor = query.state.dataUpdatedAt ?? Date.now()
  return Math.min(interval, Math.max(1_000, expiry - anchor - LOCAL_QUOTE_REFRESH_HEADROOM_MS))
}
