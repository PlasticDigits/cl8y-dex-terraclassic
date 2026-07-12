import { getCachedTokenEntry, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { lookupByTokenId } from '@/utils/tokenRegistry'

/** Minimum characters before a typed token search filters (unless query looks like a Terra address). */
export const TOKEN_SEARCH_MIN_CHARS = 2

/** Debounce delay for Swap token combobox queries (GitLab #481). */
export const TOKEN_SEARCH_DEBOUNCE_MS = 300

/** Cap for filtered (typed) search hits. Empty query browses the full allowed list. */
export const TOKEN_SEARCH_RESULT_LIMIT = 20

/** Ignore / truncate pasted queries longer than this (client DoS guard). */
export const TOKEN_SEARCH_MAX_QUERY_LENGTH = 128

const TERRA_ADDRESS_PREFIX = 'terra1'

/** Whether the query is long enough to filter (empty query is allowed — shows full allowed list). */
export function isTokenSearchQueryReady(raw: string): boolean {
  const q = normalizeTokenSearchQuery(raw)
  if (!q) return true
  if (q.toLowerCase().startsWith(TERRA_ADDRESS_PREFIX) && q.length >= 20) return true
  return q.length >= TOKEN_SEARCH_MIN_CHARS
}

/** Trim and truncate oversized paste into the token search field. */
export function normalizeTokenSearchQuery(raw: string): string {
  return raw.trim().slice(0, TOKEN_SEARCH_MAX_QUERY_LENGTH)
}

/**
 * Searchable text for a factory-routable token id (denom or CW20 address).
 * Includes display symbol, localStorage-cached CW20 metadata, and registry entries.
 */
export function buildTokenLocalSearchHaystack(tokenId: string): string {
  const parts = [tokenId, getTokenDisplaySymbol(tokenId)]
  const cached = getCachedTokenEntry(tokenId)
  if (cached?.symbol) parts.push(cached.symbol)
  if (cached?.name) parts.push(cached.name)
  const reg = lookupByTokenId(tokenId)
  if (reg) {
    parts.push(reg.symbol, reg.name)
  }
  return parts.filter(Boolean).join(' ').toLowerCase()
}

function tokenMatchesLocalQuery(tokenId: string, query: string): boolean {
  const q = normalizeTokenSearchQuery(query).toLowerCase()
  if (!q) return true
  return buildTokenLocalSearchHaystack(tokenId).includes(q)
}

function compareTokenIdsBySymbol(a: string, b: string): number {
  const sa = getTokenDisplaySymbol(a).toLowerCase()
  const sb = getTokenDisplaySymbol(b).toLowerCase()
  const cmp = sa.localeCompare(sb)
  if (cmp !== 0) return cmp
  return a.localeCompare(b)
}

/**
 * Client-side token filter for Swap (`TokenSearchSelect`).
 * Only emits ids from `tokens` (factory gate). Empty query returns the full allowed set
 * sorted by display symbol; typed queries cap at {@link TOKEN_SEARCH_RESULT_LIMIT}.
 */
export function filterTokensByLocalSearch(
  tokens: string[],
  query: string,
  options?: { excludeToken?: string; limit?: number }
): string[] {
  const exclude = options?.excludeToken
  const allowed = tokens.filter((t) => t !== exclude)
  const q = normalizeTokenSearchQuery(query)

  // Empty or too-short query: browse the full allowed list (sorted). Filtering starts at min chars.
  if (!isTokenSearchQueryReady(q) || !q) {
    return [...allowed].sort(compareTokenIdsBySymbol)
  }

  const limit = options?.limit ?? TOKEN_SEARCH_RESULT_LIMIT
  return allowed
    .filter((t) => tokenMatchesLocalQuery(t, q))
    .sort(compareTokenIdsBySymbol)
    .slice(0, limit)
}
