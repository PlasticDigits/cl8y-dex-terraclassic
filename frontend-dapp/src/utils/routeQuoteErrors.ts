import { LOCAL_QUOTE_SIGNING_WINDOW_MS } from '@/utils/localQuoteWindow'

/** Public local-quote outcomes (#1473). No remote text or executable plan crosses this boundary. */
export const ROUTE_QUOTE_REASONS = {
  protection: 'This amount exceeds the selected price protection.',
  price_impact_too_high: 'Fair-reference deviation is too high for this trade.',
  amount_too_small: 'This amount is too small to receive a positive output.',
  amount_too_large: 'This amount exceeds the supported trade limits.',
  insufficient_liquidity: 'There is insufficient liquidity for this amount.',
  unsupported_semantics: 'This trade is not supported by current pricing.',
  reference_policy_excluded: 'Reliable reference pricing is unavailable for this market.',
  missing_observations: 'Pricing is temporarily unavailable for this trade.',
  stale_observations: 'Pricing has expired; a fresh quote is needed.',
  accounting_unavailable: 'Pricing could not be calculated safely for this trade.',
  trading_blocked: 'Trading is restricted for this route.',
  no_route: 'No supported route connects these tokens.',
  invalid_request: 'Check the trade amount and settings.',
} as const

export type RouteQuoteReason = keyof typeof ROUTE_QUOTE_REASONS
export const MAX_ROUTE_ERROR_BYTES = 4096

export class IndexerApiError extends Error {
  readonly status: number
  readonly quoteReason?: RouteQuoteReason
  readonly indicativeAmountOut?: string
  readonly indicativeExpiresAtMs?: number
  readonly quoteGeneration?: string
  constructor(
    status: number,
    quoteReason?: RouteQuoteReason,
    indicativeAmountOut?: string,
    indicativeExpiresAtMs?: number,
    quoteGeneration?: string
  ) {
    // Status text and bodies are untrusted. Keep the compatibility classifier prefix fixed.
    super(`Indexer API error: ${status}`)
    this.name = 'IndexerApiError'
    this.status = status
    this.quoteReason = quoteReason
    this.indicativeAmountOut = indicativeAmountOut
    this.indicativeExpiresAtMs = indicativeExpiresAtMs
    this.quoteGeneration = quoteGeneration
  }
}

/** Strictly positive canonical Uint128; bounded before BigInt parsing. */
function positiveRaw(value: unknown): value is string {
  return typeof value === 'string' && /^[1-9]\d{0,38}$/.test(value) && BigInt(value) <= (1n << 128n) - 1n
}

export function parseRouteQuoteError(status: number, text: string, nowMs = Date.now()): IndexerApiError {
  const fallback = new IndexerApiError(status)
  if (text.length > MAX_ROUTE_ERROR_BYTES) return fallback
  try {
    const body: unknown = JSON.parse(text)
    if (!body || typeof body !== 'object' || Array.isArray(body)) return fallback
    const data = body as Record<string, unknown>
    if (typeof data.error !== 'string' || !Object.hasOwn(ROUTE_QUOTE_REASONS, data.error)) return fallback
    const reason = data.error as RouteQuoteReason
    const expectedStatus = ['missing_observations', 'stale_observations', 'accounting_unavailable'].includes(reason)
      ? 503
      : reason === 'invalid_request'
        ? 400
        : 422
    if (status !== expectedStatus) return fallback
    // An estimate has no operations, floors or signing metadata. Expiry is original observation expiry.
    const expires = data.expires_at_ms
    const observed = data.observed_at_ms
    const generation =
      typeof data.generation === 'string' && /^[a-zA-Z0-9:_-]{1,256}$/.test(data.generation)
        ? data.generation
        : undefined
    const indicative =
      reason === 'protection' &&
      data.execution_supported === false &&
      positiveRaw(data.indicative_amount_out) &&
      typeof expires === 'number' &&
      Number.isSafeInteger(expires) &&
      expires > nowMs &&
      expires <= nowMs + LOCAL_QUOTE_SIGNING_WINDOW_MS &&
      typeof observed === 'number' &&
      Number.isSafeInteger(observed) &&
      observed > 0 &&
      observed <= nowMs &&
      expires > observed &&
      expires - observed <= LOCAL_QUOTE_SIGNING_WINDOW_MS &&
      generation != null
        ? data.indicative_amount_out
        : undefined
    return new IndexerApiError(status, reason, indicative, indicative ? (expires as number) : undefined, generation)
  } catch {
    return fallback
  }
}

/** Display only for the current settled intent; callers never insert this into quote/signing data. */
export function currentIndicativeAmount(err: unknown, currentIntent: boolean, nowMs = Date.now()): string | undefined {
  return currentIntent &&
    err instanceof IndexerApiError &&
    err.indicativeExpiresAtMs != null &&
    err.indicativeExpiresAtMs > nowMs
    ? err.indicativeAmountOut
    : undefined
}

export function humanizeRouteQuoteError(err: unknown): string | undefined {
  return err instanceof IndexerApiError && err.quoteReason ? ROUTE_QUOTE_REASONS[err.quoteReason] : undefined
}
