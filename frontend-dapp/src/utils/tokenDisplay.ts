import { queryContract } from '@/services/terraclassic/queries'
import type { AssetInfo } from '@/types'
import { isPairLegDecimals } from './formatAmount'
import { parseTokenInfoDecimals } from './swapAssetDecimals'
import { lookupByTokenId, lookupByAssetInfo, registryProductSymbol } from './tokenRegistry'

export interface CW20TokenInfo {
  name: string
  symbol: string
  /** Parsed 0…18, or null when LCD omitted / hostile decimals (#1255). */
  decimals: number | null
  /** True when LCD sent a `decimals` field outside 0…18. */
  decimalsHostile: boolean
  total_supply: string
}

/** Versioned so pre-#1255 `{symbol,name}` rows cannot be read as “decimals unknown → 6”. */
export const CW20_TOKEN_INFO_CACHE_KEY = 'cl8y-dex-token-info-v2'

type CachedEntry = { symbol: string; name: string; decimals?: number; decimalsHostile?: boolean }

function loadCache(): Record<string, CachedEntry> {
  try {
    return JSON.parse(localStorage.getItem(CW20_TOKEN_INFO_CACHE_KEY) || '{}')
  } catch {
    return {}
  }
}

function saveCache(cache: Record<string, CachedEntry>) {
  try {
    localStorage.setItem(CW20_TOKEN_INFO_CACHE_KEY, JSON.stringify(cache))
  } catch {
    // quota exceeded
  }
}

const inFlightQueries = new Map<string, Promise<CW20TokenInfo | null>>()

/** Vitest: drop hung LCD promises so a later mock is not stuck behind T7. */
export function resetCw20TokenInfoInFlightForTests() {
  inFlightQueries.clear()
}

export function getCachedTokenSymbol(tokenId: string): string | null {
  const reg = lookupByTokenId(tokenId)
  if (reg) return reg.symbol
  const cache = loadCache()
  return cache[tokenId.toLowerCase()]?.symbol ?? null
}

/** Cached CW20 metadata from prior on-chain `token_info` reads (localStorage). */
export function getCachedTokenEntry(tokenId: string): CachedEntry | null {
  const reg = lookupByTokenId(tokenId)
  if (reg) return { symbol: reg.symbol, name: reg.name, decimals: reg.decimals }
  const cache = loadCache()
  const entry = cache[tokenId.toLowerCase()]
  if (!entry) return null
  if (entry.decimals != null && !isPairLegDecimals(entry.decimals)) {
    return { symbol: entry.symbol, name: entry.name, decimalsHostile: entry.decimalsHostile }
  }
  return entry
}

/** Cached LCD `token_info.decimals` when in 0…18. Missing field is unresolved, not 6. */
export function getCachedTokenDecimals(tokenId: string): number | null {
  const entry = getCachedTokenEntry(tokenId)
  return isPairLegDecimals(entry?.decimals) ? entry.decimals : null
}

export function getCachedTokenDecimalsHostile(tokenId: string): boolean {
  const cache = loadCache()
  return cache[tokenId.toLowerCase()]?.decimalsHostile === true
}

export function getTokenLogoURI(info: AssetInfo): string | undefined {
  return lookupByAssetInfo(info)?.logoURI
}

export async function fetchCW20TokenInfo(contractAddr: string): Promise<CW20TokenInfo | null> {
  const key = contractAddr.toLowerCase()
  const existing = inFlightQueries.get(key)
  if (existing) return existing

  const promise = queryContract<{
    name: string
    symbol: string
    decimals: unknown
    total_supply: string
  }>(contractAddr, { token_info: {} })
    .then((info) => {
      const decimals = parseTokenInfoDecimals(info.decimals)
      const decimalsHostile = info.decimals !== undefined && info.decimals !== null && decimals == null
      const cache = loadCache()
      cache[key] = {
        symbol: info.symbol,
        name: info.name,
        ...(decimals != null ? { decimals } : {}),
        ...(decimalsHostile ? { decimalsHostile: true } : {}),
      }
      saveCache(cache)
      inFlightQueries.delete(key)
      return {
        name: info.name,
        symbol: info.symbol,
        decimals,
        decimalsHostile,
        total_supply: info.total_supply,
      }
    })
    .catch(() => {
      inFlightQueries.delete(key)
      return null
    })

  inFlightQueries.set(key, promise)
  return promise
}

export function getTokenDisplaySymbol(tokenId: string): string {
  if (!tokenId?.trim()) return ''
  const product = registryProductSymbol(tokenId)
  if (product) return product
  if (tokenId.toLowerCase().startsWith('terra1') && tokenId.length >= 44) {
    const cache = loadCache()
    if (cache[tokenId.toLowerCase()]?.symbol) return cache[tokenId.toLowerCase()].symbol
    return shortenAddress(tokenId)
  }
  return tokenId
}

export function shortenAddress(addr: string, startChars = 8, endChars = 6): string {
  if (addr.length <= startChars + endChars + 2) return addr
  return `${addr.slice(0, startChars)}…${addr.slice(-endChars)}`
}

/** Visible trader chip prefix (GitLab #656). Do not change `shortenAddress` defaults. */
export const TRADER_ADDR_START_CHARS = 4
/** Visible trader chip suffix (GitLab #656). */
export const TRADER_ADDR_END_CHARS = 6

/** 4/6 bech32 chip for trader-as-person surfaces. Defaults stay 8/6 for contracts. */
export function shortenTraderAddress(addr: string): string {
  if (!addr) return addr
  return shortenAddress(addr, TRADER_ADDR_START_CHARS, TRADER_ADDR_END_CHARS)
}

export function isAddressLike(s: string): boolean {
  return (s.startsWith('terra1') && s.length >= 44) || (s.startsWith('0x') && s.length >= 42)
}

export function getAddressForBlockie(info: AssetInfo): string | undefined {
  if ('token' in info) return info.token.contract_addr
  return undefined
}

const BANK_DENOM_AS_NAME = new Set(['uluna', 'uusd'])
const POOL_ASSET_NAME_MAX_WORDS = 5
const POOL_ASSET_NAME_MAX_CHARS = 48

/**
 * Indexer `name` is allowed for pool provide labels only when it is short, text-only,
 * not a bank denom, and not the same as the product ticker (GitLab #661 / #489 / A1).
 */
export function usablePoolAssetName(name: string | undefined | null, symbol: string): boolean {
  if (!name?.trim() || !symbol?.trim()) return false
  const trimmed = name.trim()
  if (trimmed.toLowerCase() === symbol.trim().toLowerCase()) return false
  if (BANK_DENOM_AS_NAME.has(trimmed.toLowerCase())) return false
  if (/[<>]|javascript:|on\w+\s*=/i.test(trimmed)) return false
  if (trimmed.length > POOL_ASSET_NAME_MAX_CHARS) return false
  const words = trimmed.split(/\s+/).filter(Boolean)
  return words.length > 0 && words.length <= POOL_ASSET_NAME_MAX_WORDS
}

/**
 * Visible Advanced provide field label: `{Name} ({SYMBOL})` or `{SYMBOL}`.
 * Never `UST1 (UST1)`, never `uluna` as the name, never HTML.
 */
export function formatPoolAssetFieldLabel(opts: { name?: string | null; symbol: string }): string {
  const symbol = opts.symbol.trim()
  if (!symbol) return ''
  if (usablePoolAssetName(opts.name, symbol)) {
    return `${opts.name!.trim()} (${symbol})`
  }
  return symbol
}

/** `aria-label` for a provide amount input — product ticker + "amount", never Asset A/B. */
export function poolProvideAmountAriaLabel(symbol: string): string {
  const ticker = symbol.trim()
  return ticker ? `${ticker} amount` : 'amount'
}
