/** Minimum characters before a pair search hits the indexer (unless query looks like a Terra address). */
export const PAIR_SEARCH_MIN_CHARS = 2

/** Debounce delay for pair combobox queries (GitLab #314). */
export const PAIR_SEARCH_DEBOUNCE_MS = 300

/** Default page size for combobox results. */
export const PAIR_SEARCH_RESULT_LIMIT = 20

const TERRA_ADDRESS_PREFIX = 'terra1'

/** Whether the query is long enough to search (empty query is allowed — shows default list). */
export function isPairSearchQueryReady(raw: string): boolean {
  const q = raw.trim()
  if (!q) return true
  if (q.toLowerCase().startsWith(TERRA_ADDRESS_PREFIX) && q.length >= 20) return true
  return q.length >= PAIR_SEARCH_MIN_CHARS
}

/** Client-side fallback filter when the indexer is unavailable. */
export function filterPairsByLocalSearch(
  labelsByAddress: Map<string, string>,
  query: string,
  limit = PAIR_SEARCH_RESULT_LIMIT
): string[] {
  const q = query.trim().toLowerCase()
  const entries = [...labelsByAddress.entries()]
  if (!q) return entries.slice(0, limit).map(([addr]) => addr)
  return entries
    .filter(([, label]) => label.toLowerCase().includes(q))
    .slice(0, limit)
    .map(([addr]) => addr)
}
