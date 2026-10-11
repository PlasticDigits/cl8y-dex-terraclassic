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

/** Refresh local arithmetic at expiry without polling producers or extending the quote.
 * A one-second minimum keeps near-expiry responses from creating a tight solve loop. */
export function localQuoteRefetchInterval(query: {
  state: { data?: unknown; fetchStatus: 'fetching' | 'paused' | 'idle'; error?: unknown }
}): number | false {
  const interval = simQuoteRefetchInterval(query)
  // Failed refreshes retain old data; expiry must not accelerate their retry cadence.
  if (interval === false || query.state.error != null) return interval
  const expiry = localQuoteExpiryMs(query.state.data)
  if (expiry === undefined) return interval
  if (!Number.isFinite(expiry)) return false
  return Math.min(interval, Math.max(1_000, expiry - Date.now()))
}
