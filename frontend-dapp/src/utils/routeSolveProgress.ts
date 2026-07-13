/**
 * Live indexer route-solve progress copy (GitLab #485).
 * Advisory only — does not affect submit/receive gating (#484 invariants stay in quoteDebounce).
 */

export const SIM_QUOTE_PROGRESS_MIN_VISIBLE_MS = 500
export const SIM_QUOTE_PROGRESS_POLL_MS = 1_000

export type RouteSolveProgressSnapshot = {
  stage: string
  done: number
  total: number
  label: string
  cache_hit?: boolean
}

/** Prefer indexer `label`; fall back to honest x/y path copy. */
export function formatRouteSolveSearchProgress(
  progress: Pick<RouteSolveProgressSnapshot, 'done' | 'total' | 'label'>
): string {
  const trimmed = progress.label?.trim()
  if (trimmed) return trimmed
  const total = Math.max(0, progress.total)
  const done = Math.max(0, Math.min(progress.done, total || progress.done))
  if (total > 0) return `Searching ${done} of ${total} paths…`
  return 'Searching…'
}

/**
 * Show live progress only after a short delay (avoids flicker on sub-second solves)
 * and only when the indexer reports a non-idle stage with a usable label.
 */
export function shouldShowRouteSolveProgress(
  fetchStartedAtMs: number | null,
  progress: RouteSolveProgressSnapshot | null,
  nowMs: number,
  minVisibleMs = SIM_QUOTE_PROGRESS_MIN_VISIBLE_MS
): boolean {
  if (fetchStartedAtMs == null || !progress) return false
  if (nowMs - fetchStartedAtMs < minVisibleMs) return false
  const stage = progress.stage.trim().toLowerCase()
  if (!stage || stage === 'idle') return false
  if (stage === 'done' || stage === 'cached' || stage === 'error') return false
  return Boolean(progress.label?.trim()) || progress.total > 0
}

/**
 * Display-only loading label for Swap button / receive / Trade quoting row.
 * Does not change #484 submit or receive amount gates.
 */
export function resolveSimQuoteLoadingLabel(
  isFetching: boolean,
  hasSettledQuote: boolean,
  progress: RouteSolveProgressSnapshot | null,
  fetchStartedAtMs: number | null,
  nowMs: number,
  fallback = 'Calculating...'
): string {
  if (!isFetching) return fallback
  // Background refetch: keep quiet on receive (caller should not replace amount);
  // button may still show Calculating via stale gate — prefer subtle fallback.
  if (hasSettledQuote) return fallback
  if (shouldShowRouteSolveProgress(fetchStartedAtMs, progress, nowMs)) {
    return formatRouteSolveSearchProgress(progress!)
  }
  return fallback
}
