/**
 * Swap deep-link query params (GitLab #711).
 *
 * Inbound aliases (Uniswap / Pancake / Terra DEX). First key family with a
 * non-empty value wins; within a family, the last repeated key wins (same as
 * Charts `?price=`). Hostile / overlong / wrong-chain values are ignored.
 * Factory-gate + retail gem hide happen in {@link applySwapQueryParams}.
 *
 * Never concatenates user strings into URLs — outbound helpers use `URLSearchParams`.
 */

import { isPositiveDecimalAmount } from '@/utils/decimalAmountInput'
import { isRetailHiddenTestToken } from '@/utils/pairCatalogRank'
import { isValidTerraBech32Address } from '@/utils/terraAddressValidation'
import { executeIdToQueryToken, queryTokenToExecuteId } from '@/utils/tokenlistQueryCatalog'

export const SWAP_QUERY_VALUE_MAX_LEN = 80
export const SWAP_QUERY_AMOUNT_MAX_CHARS = 24

/** Pay-side keys, Uniswap-first then Terra / generic. Case-insensitive names. */
export const SWAP_QUERY_PAY_KEYS = [
  'inputCurrency',
  'from',
  'tokenIn',
  'token_in',
  'sellToken',
  'currencyIn',
  'inToken',
  'pay',
  'offer',
] as const

/** Receive-side keys. `to` is the Terra DEX token, not Uniswap `recipient`. */
export const SWAP_QUERY_RECEIVE_KEYS = [
  'outputCurrency',
  'to',
  'tokenOut',
  'token_out',
  'buyToken',
  'currencyOut',
  'outToken',
  'receive',
  'ask',
] as const

/** Optional independent-field human amount. */
export const SWAP_QUERY_AMOUNT_KEYS = ['exactAmount', 'amount', 'value', 'amountIn'] as const

/** Uniswap `exactField` / `independentField`. Only `output` is special; else pay-sided. */
export const SWAP_QUERY_EXACT_FIELD_KEYS = ['exactField', 'independentField'] as const

export type SwapExactField = 'output'

export type CanonicalSwapSearchInput = {
  payId: string
  receiveId: string
  amountHuman?: string | null
  exactField?: SwapExactField | null
}

export type SwapQueryParse = {
  payId: string | null
  receiveId: string | null
  payAmountHuman: string | null
  exactField: SwapExactField | null
}

export type AppliedSwapQuery = {
  payId: string
  receiveId: string
  payAmountHuman: string | null
}

export type ApplySwapQueryOptions = {
  /** Override production gem hide. Default: {@link isRetailHiddenTestToken}. */
  isHiddenToken?: (tokenId: string) => boolean
}

function looksHostileSwapQueryValue(raw: string): boolean {
  const t = raw.trim()
  if (!t || t.length > SWAP_QUERY_VALUE_MAX_LEN) return true
  if (/[<>"'`\\/?#\s]/.test(t)) return true
  const lower = t.toLowerCase()
  if (
    lower.startsWith('javascript:') ||
    lower.startsWith('data:') ||
    lower.startsWith('http:') ||
    lower.startsWith('https:') ||
    lower.startsWith('//')
  ) {
    return true
  }
  if (lower.startsWith('0x')) return true
  if (lower.startsWith('ibc/')) return true
  if (lower.startsWith('factory/')) return true
  return false
}

function toSearchParams(search: string | URLSearchParams | null | undefined): URLSearchParams {
  if (search == null) return new URLSearchParams()
  if (typeof search === 'string') {
    return new URLSearchParams(search.startsWith('?') ? search.slice(1) : search)
  }
  return search
}

/** Last non-empty value for a case-insensitive key name. */
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

/** First alias family with a non-empty value; last repeat within that family. */
function firstFamilyValue(params: URLSearchParams, keys: readonly string[]): string | null {
  for (const key of keys) {
    const v = lastNonEmptyForKey(params, key)
    if (v) return v
  }
  return null
}

/**
 * Resolve a query value to an execute id (`uluna` / `uusd` / checksummed `terra1`).
 * Order: hostile / EVM → native denom → `UST` alias → unique tokenlist symbol → terra1.
 * Unknown / hostile → `null` (never echo). LCD `token_info.symbol` is not consulted (**X1**).
 */
export function resolveSwapQueryTokenValue(raw: string | null | undefined): string | null {
  if (raw == null) return null
  const trimmed = raw.trim()
  if (!trimmed || looksHostileSwapQueryValue(trimmed)) return null
  const lower = trimmed.toLowerCase()
  if (lower === 'eth' || lower === 'bnb' || lower === 'weth') return null
  if (lower === 'uluna' || lower === 'uusd') return lower
  if (lower === 'ust') return 'uusd'
  const fromList = queryTokenToExecuteId(trimmed)
  if (fromList) return fromList
  if (lower.startsWith('terra1')) {
    if (!isValidTerraBech32Address(trimmed)) return null
    return trimmed
  }
  return null
}

function parseAmountValue(raw: string | null): string | null {
  if (raw == null) return null
  const trimmed = raw.trim()
  if (!trimmed || trimmed.length > SWAP_QUERY_AMOUNT_MAX_CHARS) return null
  if (!isPositiveDecimalAmount(trimmed)) return null
  return trimmed
}

function parseExactFieldValue(raw: string | null): SwapExactField | null {
  if (raw == null) return null
  const trimmed = raw.trim().toLowerCase()
  if (trimmed === 'output') return 'output'
  return null
}

/** Inbound `exactField=output` (and Uniswap `independentField=output`). Other values → pay-sided. */
export function parseSwapExactField(search: string | URLSearchParams | null | undefined): SwapExactField | null {
  const params = toSearchParams(search)
  return parseExactFieldValue(firstFamilyValue(params, SWAP_QUERY_EXACT_FIELD_KEYS))
}

/**
 * First-party outbound search: `from` / `to` plus optional `exactAmount` / `exactField=output`.
 * Unique tokenlist symbols when known (`LUNC`, `UST1`); else execute id. Uniswap aliases
 * are inbound-only. Empty / illegal amount is omitted.
 */
export function canonicalSwapSearch(input: CanonicalSwapSearchInput): URLSearchParams {
  const q = new URLSearchParams()
  q.set('from', executeIdToQueryToken(input.payId))
  q.set('to', executeIdToQueryToken(input.receiveId))
  const amount = input.amountHuman?.trim()
  if (amount && isPositiveDecimalAmount(amount) && amount.length <= SWAP_QUERY_AMOUNT_MAX_CHARS) {
    q.set('exactAmount', amount)
  }
  if (input.exactField === 'output') {
    q.set('exactField', 'output')
  }
  return q
}

/** True when two search strings encode the same canonical Swap query (order-sensitive `URLSearchParams`). */
export function swapSearchEquals(
  a: string | URLSearchParams | null | undefined,
  b: string | URLSearchParams | null | undefined
): boolean {
  return toSearchParams(a).toString() === toSearchParams(b).toString()
}

/**
 * Parse search into resolved ids + optional amount + exactField. Does **not** factory-gate
 * or hide gems — call {@link applySwapQueryParams} before setting Swap state.
 */
export function parseSwapQueryParams(search: string | URLSearchParams | null | undefined): SwapQueryParse {
  const params = toSearchParams(search)
  const payRaw = firstFamilyValue(params, SWAP_QUERY_PAY_KEYS)
  const receiveRaw = firstFamilyValue(params, SWAP_QUERY_RECEIVE_KEYS)
  const amountRaw = firstFamilyValue(params, SWAP_QUERY_AMOUNT_KEYS)
  return {
    payId: resolveSwapQueryTokenValue(payRaw),
    receiveId: resolveSwapQueryTokenValue(receiveRaw),
    payAmountHuman: parseAmountValue(amountRaw),
    exactField: parseExactFieldValue(firstFamilyValue(params, SWAP_QUERY_EXACT_FIELD_KEYS)),
  }
}

function canonicalFactoryTokenId(factoryTokenIds: readonly string[], id: string): string | null {
  const lower = id.trim().toLowerCase()
  if (!lower) return null
  const hit = factoryTokenIds.find((t) => t.trim().toLowerCase() === lower)
  return hit ?? null
}

function otherDefault(defaults: readonly [string, string], taken: string): string {
  const t = taken.trim().toLowerCase()
  const a = defaults[0]
  const b = defaults[1]
  if (a.trim().toLowerCase() === t) return b
  return a
}

/**
 * Factory-gate + gem hide + same-token / one-sided defaults.
 * Pay and receive always differ when `defaults` are two distinct ids.
 */
export function applySwapQueryParams(
  search: string | URLSearchParams | null | undefined,
  factoryTokenIds: readonly string[],
  defaults: readonly [string, string],
  options?: ApplySwapQueryOptions
): AppliedSwapQuery {
  const isHidden = options?.isHiddenToken ?? isRetailHiddenTestToken
  const parsed = parseSwapQueryParams(search)

  const allow = (id: string | null): string | null => {
    if (!id) return null
    const canon = canonicalFactoryTokenId(factoryTokenIds, id)
    if (!canon) return null
    if (isHidden(canon)) return null
    return canon
  }

  let pay = allow(parsed.payId)
  let receive = allow(parsed.receiveId)

  if (pay && receive && pay.trim().toLowerCase() === receive.trim().toLowerCase()) {
    receive = null
  }

  if (!pay && !receive) {
    return { payId: defaults[0], receiveId: defaults[1], payAmountHuman: parsed.payAmountHuman }
  }
  if (pay && !receive) {
    receive = otherDefault(defaults, pay)
    if (receive.trim().toLowerCase() === pay.trim().toLowerCase()) {
      receive = defaults[1]
    }
  } else if (receive && !pay) {
    pay = otherDefault(defaults, receive)
    if (pay.trim().toLowerCase() === receive.trim().toLowerCase()) {
      pay = defaults[0]
    }
  }

  if (pay && receive && pay.trim().toLowerCase() === receive.trim().toLowerCase()) {
    return { payId: defaults[0], receiveId: defaults[1], payAmountHuman: parsed.payAmountHuman }
  }

  return {
    payId: pay ?? defaults[0],
    receiveId: receive ?? defaults[1],
    payAmountHuman: parsed.payAmountHuman,
  }
}

/**
 * First-party Swap href (`/?from=&to=`). Prefers unique tokenlist symbols (#715).
 */
export function swapDeepLinkPath(
  payId: string,
  receiveId: string,
  amountHuman?: string | null,
  exactField?: SwapExactField | null
): string {
  const q = canonicalSwapSearch({ payId, receiveId, amountHuman, exactField })
  return `/?${q.toString()}`
}
