/** Debounce before swap / market sim queries fire (GitLab #346). */
export const SIM_QUOTE_DEBOUNCE_MS = 350

/**
 * Periodic sim refresh interval (Swap + Trade market).
 * Must be paired with {@link simQuoteRefetchInterval} — a bare `10_000` lets React Query's
 * default `cancelRefetch` abort in-flight quotes every 10s, which never settles for slow
 * multi-hop indexer solves (GitLab #484).
 */
export const SIM_QUOTE_REFETCH_INTERVAL_MS = 10_000

/**
 * `refetchInterval` callback for sim quotes: skip scheduling while a fetch is in flight so a
 * slow `queryFn` (indexer route solve + LCD enrich/sim/preflight) can finish instead of being
 * cancelled and restarted (GitLab #484).
 */
export function simQuoteRefetchInterval(query: {
  state: { fetchStatus: 'fetching' | 'paused' | 'idle' }
}): number | false {
  if (query.state.fetchStatus === 'fetching') return false
  return SIM_QUOTE_REFETCH_INTERVAL_MS
}

/**
 * Receive-field "Calculating..." gate for Swap / Trade market.
 *
 * - **Same pay inputs, background refetch (#484):** when a quote for the *current* query key
 *   already settled (`hasSettledQuote` and not placeholder), keep the prior amount visible
 *   even while `isFetching` — do not pulse Calculating on every 10s refresh.
 * - **Pay amount / token (query key) change (#496):** treat prior amount as stale. Show
 *   Calculating while typed pay raw ≠ debounced sim key (`payInputsPending`), or while React
 *   Query is still showing `keepPreviousData` for a previous key (`isPlaceholderData`).
 * - **First load:** Calculating while fetching with no settled quote yet.
 *
 * Submit stays blocked separately via {@link isSubmitQuoteStale} / `isFetching` (#356).
 *
 * @param hasSettledQuote Prefer `!!data && !isPlaceholderData` so placeholder rows from a
 *   prior key are not treated as settled for the current inputs.
 */
export function shouldShowSimReceiveCalculating(
  isFetching: boolean,
  hasSettledQuote: boolean,
  isPlaceholderData = false,
  payInputsPending = false
): boolean {
  if (payInputsPending || isPlaceholderData) return true
  return isFetching && !hasSettledQuote
}

export type SubmitHybridSnapshot = {
  bookInputHuman: string
  hybridMaxMakers: number
}

export type SubmitHybridStaleInput = {
  enabled: boolean
  live: SubmitHybridSnapshot
  snapshotted: SubmitHybridSnapshot
}

/**
 * Block submit while typed pay size differs from the debounced sim key, prior quote is still shown,
 * a fetch is in flight for the active debounced key (GitLab #356), or hybrid book/max-makers differ
 * from the snapshotted sim key (GitLab #360).
 */
export function isSubmitQuoteStale(
  rawInputAmount: string,
  debouncedRawInputAmount: string,
  isPlaceholderData: boolean,
  isFetching: boolean,
  hybrid?: SubmitHybridStaleInput
): boolean {
  if (rawInputAmount !== debouncedRawInputAmount || isPlaceholderData || isFetching) {
    return true
  }
  if (hybrid?.enabled) {
    if (hybrid.live.bookInputHuman !== hybrid.snapshotted.bookInputHuman) return true
    if (hybrid.live.hybridMaxMakers !== hybrid.snapshotted.hybridMaxMakers) return true
  }
  return false
}

/** @deprecated Prefer {@link isSubmitQuoteStale} — kept for import stability during #356 rollout. */
export const isSimQuoteStaleForSubmit = (
  rawInputAmount: string,
  debouncedRawInputAmount: string,
  isPlaceholderData: boolean,
  isFetching = false
): boolean => isSubmitQuoteStale(rawInputAmount, debouncedRawInputAmount, isPlaceholderData, isFetching)

/** Defensive guard inside mutationFn — submit UI should already block stale quotes. */
export function assertSubmitQuotePayRawAligned(rawInputAmount: string, debouncedRawInputAmount: string): void {
  if (rawInputAmount !== debouncedRawInputAmount) {
    throw new Error('Quote still updating; wait for the quote to settle before submitting.')
  }
}

/** Defensive guard for hybrid book leg + max makers (#360). */
export function assertSubmitHybridAligned(live: SubmitHybridSnapshot, snapshotted: SubmitHybridSnapshot): void {
  if (live.bookInputHuman !== snapshotted.bookInputHuman || live.hybridMaxMakers !== snapshotted.hybridMaxMakers) {
    throw new Error('Quote still updating; wait for the quote to settle before submitting.')
  }
}

export type SubmitAlignedSimPayload<T extends { return_amount: string; indexerOperations?: unknown }> = {
  payRaw: string
  minReceived: string | null
  simData: T
  indexerOperations: T['indexerOperations']
}

/** Bundle pay size + quote-derived fields for a single submit snapshot (GitLab #356). */
export function buildSubmitAlignedSimPayload<T extends { return_amount: string; indexerOperations?: unknown }>(
  payRaw: string,
  simData: T,
  slippageTolerance: number,
  applySlippageFloor: (returnAmount: string, slippagePct: number) => string | null
): SubmitAlignedSimPayload<T> {
  return {
    payRaw,
    minReceived: applySlippageFloor(simData.return_amount, slippageTolerance),
    simData,
    indexerOperations: simData.indexerOperations,
  }
}
