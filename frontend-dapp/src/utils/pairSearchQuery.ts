import type { PairInfo } from '@/types'
import { assetInfoLabel } from '@/types'
import { pairInfoMenuLabel, type PairMenuLabelVariant } from '@/utils/pairMenuOptions'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'

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

/** Split `XXX YYY` or `XXX/YYY` pair-symbol queries into two lowercase tokens. */
export function parsePairSymbolQueryTokens(q: string): [string, string] | null {
  const trimmed = q.trim()
  const parts: string[] = trimmed.includes('/') ? trimmed.split('/') : trimmed.split(/\s+/).filter(Boolean)
  if (parts.length !== 2) return null
  const t0 = parts[0].trim().toLowerCase()
  const t1 = parts[1].trim().toLowerCase()
  if (!t0 || !t1) return null
  return [t0, t1]
}

function legSearchTokens(info: PairInfo['asset_infos'][number]): string[] {
  const id = assetInfoLabel(info)
  const symbol = getTokenDisplaySymbol(id).toLowerCase()
  return [id.toLowerCase(), symbol]
}

function legMatchesToken(info: PairInfo['asset_infos'][number], token: string): boolean {
  const t = token.toLowerCase()
  return legSearchTokens(info).some((part) => part.includes(t) || t.includes(part))
}

/** Searchable text for a factory pair (labels, symbols, contract/denom ids). */
export function pairInfoSearchHaystack(pair: PairInfo, variant: PairMenuLabelVariant = 'full'): string {
  const label = pairInfoMenuLabel(pair, { variant }).toLowerCase()
  const legs = pair.asset_infos.flatMap(legSearchTokens)
  return [label, pair.contract_addr.toLowerCase(), ...legs].join(' ')
}

function pairMatchesLocalQuery(pair: PairInfo, query: string, variant: PairMenuLabelVariant): boolean {
  const q = query.trim().toLowerCase()
  if (!q) return true
  const haystack = pairInfoSearchHaystack(pair, variant)
  if (haystack.includes(q)) return true
  const tokens = parsePairSymbolQueryTokens(query)
  if (!tokens) return false
  const [t0, t1] = tokens
  const [a0, a1] = pair.asset_infos
  return (legMatchesToken(a0, t0) && legMatchesToken(a1, t1)) || (legMatchesToken(a0, t1) && legMatchesToken(a1, t0))
}

/**
 * Client-side fallback filter when the indexer is unavailable.
 * Searches menu labels, display symbols, and contract/denom identifiers.
 */
export function filterFactoryPairsByLocalSearch(
  pairs: PairInfo[],
  query: string,
  limit = PAIR_SEARCH_RESULT_LIMIT,
  variant: PairMenuLabelVariant = 'full'
): PairInfo[] {
  const q = query.trim()
  if (!q) return pairs.slice(0, limit)
  return pairs.filter((p) => pairMatchesLocalQuery(p, q, variant)).slice(0, limit)
}

/** @deprecated Prefer {@link filterFactoryPairsByLocalSearch} — label-only filter. */
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
