/**
 * `/trade?from=&to=` → unique factory pair path (GitLab #713).
 * Navigate only to `/trade/{bech32}` — never concatenate user strings into `to`.
 */
import { assetInfoLabel, type PairInfo } from '@/types'
import { isPositiveDecimalAmount } from '@/utils/decimalAmountInput'
import { isRetailHiddenTestToken } from '@/utils/pairCatalogRank'
import { parseSwapQueryParams, SWAP_QUERY_AMOUNT_MAX_CHARS, type SwapQueryParse } from '@/utils/swapQueryParams'
import { isTradePairRouteParam } from '@/utils/tradePairRoute'

export type TradeQueryTicketPrefill = {
  amountHuman: string | null
  side: 'bid' | 'ask' | null
}

export type ResolveTradePairFromQueryOptions = {
  isHiddenToken?: (tokenId: string) => boolean
}

function pairHasHiddenLeg(pair: PairInfo, isHiddenToken: (tokenId: string) => boolean): boolean {
  const legs = pairLegIds(pair)
  if (!legs) return true
  return isHiddenToken(legs[0]) || isHiddenToken(legs[1])
}

function pairLegIds(pair: PairInfo): [string, string] | null {
  const infos = pair.asset_infos
  if (!infos || infos.length < 2) return null
  const a = assetInfoLabel(infos[0]!)
  const b = assetInfoLabel(infos[1]!)
  if (!a || !b) return null
  return [a, b]
}

function sameId(a: string, b: string): boolean {
  return a.trim().toLowerCase() === b.trim().toLowerCase()
}

function pairMatchesLegs(pair: PairInfo, fromId: string, toId: string): boolean {
  const legs = pairLegIds(pair)
  if (!legs) return false
  const [a, b] = legs
  return (sameId(a, fromId) && sameId(b, toId)) || (sameId(a, toId) && sameId(b, fromId))
}

/**
 * Unique factory-listed pair whose legs match Swap-style `from`/`to` (order independent).
 * Zero or multiple matches → `null` (do not pick at random). Gems / hostile / same-token → `null`.
 */
export function resolveTradePairFromQuery(
  search: string | URLSearchParams | null | undefined,
  pairs: readonly PairInfo[],
  options?: ResolveTradePairFromQueryOptions
): string | null {
  const isHiddenToken = options?.isHiddenToken ?? isRetailHiddenTestToken
  const parsed: SwapQueryParse = parseSwapQueryParams(search)
  const fromId = parsed.payId
  const toId = parsed.receiveId
  if (!fromId || !toId) return null
  if (sameId(fromId, toId)) return null
  if (isHiddenToken(fromId) || isHiddenToken(toId)) return null

  const hits: string[] = []
  for (const pair of pairs) {
    if (!isTradePairRouteParam(pair.contract_addr)) continue
    if (pairHasHiddenLeg(pair, isHiddenToken)) continue
    if (!pairMatchesLegs(pair, fromId, toId)) continue
    hits.push(pair.contract_addr)
  }
  if (hits.length !== 1) return null
  return hits[0] ?? null
}

function parseSide(raw: string | null): 'bid' | 'ask' | null {
  if (!raw) return null
  const lower = raw.trim().toLowerCase()
  if (lower === 'buy' || lower === 'bid') return 'bid'
  if (lower === 'sell' || lower === 'ask') return 'ask'
  return null
}

function lastNonEmptyForKey(params: URLSearchParams, key: string): string | null {
  const want = key.toLowerCase()
  let last: string | null = null
  for (const [k, v] of params.entries()) {
    if (k.toLowerCase() !== want) continue
    const trimmed = v.trim()
    if (trimmed) last = trimmed
  }
  return last
}

/**
 * Optional ticket prefill after pair resolve. Never Place. Amount uses the Swap amount family.
 */
export function parseTradeTicketPrefill(search: string | URLSearchParams | null | undefined): TradeQueryTicketPrefill {
  const parsed = parseSwapQueryParams(search)
  const params =
    search instanceof URLSearchParams
      ? search
      : new URLSearchParams(typeof search === 'string' ? (search.startsWith('?') ? search.slice(1) : search) : '')
  const amount = parsed.payAmountHuman
  const amountOk =
    amount && amount.length <= SWAP_QUERY_AMOUNT_MAX_CHARS && isPositiveDecimalAmount(amount) ? amount : null
  return {
    amountHuman: amountOk,
    side: parseSide(lastNonEmptyForKey(params, 'side')),
  }
}
