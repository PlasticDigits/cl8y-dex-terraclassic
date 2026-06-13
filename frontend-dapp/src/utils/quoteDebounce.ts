/** Debounce before swap / market sim queries fire (GitLab #346). */
export const SIM_QUOTE_DEBOUNCE_MS = 350

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
