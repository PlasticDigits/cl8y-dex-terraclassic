import { queryContract } from '@/services/terraclassic/queries'
import type { AssetInfo } from '@/types'
import { lookupByTokenId, lookupByAssetInfo, registryProductSymbol } from './tokenRegistry'

interface CW20TokenInfo {
  name: string
  symbol: string
  decimals: number
  total_supply: string
}

const CACHE_KEY = 'cl8y-dex-token-info'

type CachedEntry = { symbol: string; name: string }

function loadCache(): Record<string, CachedEntry> {
  try {
    return JSON.parse(localStorage.getItem(CACHE_KEY) || '{}')
  } catch {
    return {}
  }
}

function saveCache(cache: Record<string, CachedEntry>) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(cache))
  } catch {
    // quota exceeded
  }
}

const inFlightQueries = new Map<string, Promise<CW20TokenInfo | null>>()

export function getCachedTokenSymbol(tokenId: string): string | null {
  const reg = lookupByTokenId(tokenId)
  if (reg) return reg.symbol
  const cache = loadCache()
  return cache[tokenId.toLowerCase()]?.symbol ?? null
}

/** Cached CW20 metadata from prior on-chain `token_info` reads (localStorage). */
export function getCachedTokenEntry(tokenId: string): CachedEntry | null {
  const reg = lookupByTokenId(tokenId)
  if (reg) return { symbol: reg.symbol, name: reg.name }
  const cache = loadCache()
  return cache[tokenId.toLowerCase()] ?? null
}

export function getTokenLogoURI(info: AssetInfo): string | undefined {
  return lookupByAssetInfo(info)?.logoURI
}

export async function fetchCW20TokenInfo(contractAddr: string): Promise<CW20TokenInfo | null> {
  const key = contractAddr.toLowerCase()
  const existing = inFlightQueries.get(key)
  if (existing) return existing

  const promise = queryContract<CW20TokenInfo>(contractAddr, { token_info: {} })
    .then((info) => {
      const cache = loadCache()
      cache[key] = { symbol: info.symbol, name: info.name }
      saveCache(cache)
      inFlightQueries.delete(key)
      return info
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
