import type { PairInfo } from '@/types'
import { assetInfoLabel } from '@/types'
import { getCachedTokenEntry, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { lookupByAssetInfo } from '@/utils/tokenRegistry'
import { pairInfoMenuLabel, type PairMenuLabelVariant } from '@/utils/pairMenuOptions'

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

/**
 * Searchable text for degraded pair search (menu label + symbols/names/addresses).
 * Includes localStorage-cached CW20 metadata so typed symbol search works when the indexer is down.
 */
export function buildPairLocalSearchHaystack(pair: PairInfo, menuLabel: string): string {
  const parts = [menuLabel, pair.contract_addr]
  for (const info of pair.asset_infos) {
    const id = assetInfoLabel(info)
    parts.push(id, getTokenDisplaySymbol(id))
    const cached = getCachedTokenEntry(id)
    if (cached?.symbol) parts.push(cached.symbol)
    if (cached?.name) parts.push(cached.name)
    const reg = lookupByAssetInfo(info)
    if (reg) {
      parts.push(reg.symbol, reg.name)
    }
  }
  return parts.filter(Boolean).join(' ')
}

/** Build per-pair haystacks for {@link filterPairsByLocalSearch}. */
export function buildPairSearchHaystacksByAddress(
  factoryPairs: PairInfo[],
  variant: PairMenuLabelVariant = 'full'
): Map<string, string> {
  const map = new Map<string, string>()
  for (const p of factoryPairs) {
    map.set(p.contract_addr, buildPairLocalSearchHaystack(p, pairInfoMenuLabel(p, { variant })))
  }
  return map
}

/** Client-side fallback filter when the indexer is unavailable. */
export function filterPairsByLocalSearch(
  searchHaystackByAddress: Map<string, string>,
  query: string,
  limit = PAIR_SEARCH_RESULT_LIMIT
): string[] {
  const q = query.trim().toLowerCase()
  const entries = [...searchHaystackByAddress.entries()]
  if (!q) return entries.slice(0, limit).map(([addr]) => addr)
  return entries
    .filter(([, haystack]) => haystack.toLowerCase().includes(q))
    .slice(0, limit)
    .map(([addr]) => addr)
}
